//! Sub-Step 10.3.1 — NER 7 段式 prompt 模板（D2.15 决策）
//!
//! ## 任务
//! 从给定中文文本块中识别 6 类核心实体（人名 / 地名 / 机构 / 时间 / 数字 / 事件）。
//!
//! ## 7 段结构
//! 1. 角色：你是中文命名实体识别专家
//! 2. 任务：识别 6 类实体
//! 3. 输入格式：JSON `{"chunk": "..."}`
//! 4. 输出格式：JSON `{"entities": [{"type", "text", "start", "end"}]}`
//! 5. 中文适配：分词边界 / 中英混合 / 繁简体
//! 6. few-shot：10 个中文示例（覆盖 6 类实体）
//! 7. 约束：仅输出 JSON / 不输出解释 / 实体 text 必须是原文子串

use super::{PromptContext, PromptTemplate, SevenSection, SevenSectionPrompt};

/// NER prompt 模板
#[derive(Debug, Clone, Default)]
pub struct NerPrompt;

impl NerPrompt {
    /// 创建一个新的 NER prompt 模板
    pub fn new() -> Self {
        Self
    }

    /// 构造 7 段内容（不含 chunk 替换，由 [`SevenSectionPrompt::build`] 完成）
    fn build_sections(context: &PromptContext) -> SevenSection {
        // 1. 角色
        let role = "## 角色\n你是一位中文命名实体识别（NER）专家，精通现代汉语语法、分词边界、中英文混合文本处理，能够准确识别文本中的人名、地名、机构、时间、数字、事件等核心实体。";

        // 2. 任务
        let task = format!(
            "## 任务\n从用户提供的文本块中识别以下 {} 类核心实体：\n\
             - PERSON（人名）：真实人物、虚构角色、历史人物等\n\
             - LOCATION（地名）：国家、城市、区域、地标等\n\
             - ORGANIZATION（机构）：公司、政府机关、学校、团队等\n\
             - TIME（时间）：日期、时刻、时间段、节日等\n\
             - NUMBER（数字）：数量、金额、百分比、计量值等\n\
             - EVENT（事件）：历史事件、会议、事故、庆典等\n\
             \n本次需识别的实体类型范围：[{}]",
            6,
            context.entity_types.join(", ")
        );

        // 3. 输入格式（含 {chunk} 占位符，由骨架替换）
        let input_format = "## 输入格式\n输入为 JSON 对象：\n\
            ```json\n\
            {\"chunk\": \"待识别的中文文本块\"}\n\
            ```\n\
            \n本次输入：\n\
            ```json\n\
            {\"chunk\": \"{chunk}\"}\n\
            ```";

        // 4. 输出格式
        let output_format = "## 输出格式\n输出严格遵循以下 JSON Schema（不允许任何额外字段或解释）：\n\
            ```json\n\
            {\n  \
              \"entities\": [\n    \
                {\n      \
                  \"type\": \"PERSON\",\n      \
                  \"text\": \"张三\",\n      \
                  \"start\": 0,\n      \
                  \"end\": 2\n    \
                }\n  \
              ]\n\
            }\n\
            ```\n\
            字段说明：\n\
            - `type`：实体类型英文枚举（PERSON / LOCATION / ORGANIZATION / TIME / NUMBER / EVENT）\n\
            - `text`：实体在原文中的字面文本（必须为原文子串，原样保留繁简体与大小写）\n\
            - `start`：实体在 `chunk` 中的字符级起始偏移（含，从 0 开始）\n\
            - `end`：实体在 `chunk` 中的字符级结束偏移（不含）";

        // 5. 中文适配
        let chinese_adaptation = "## 中文适配\n\
            - **分词边界**：中文无空格分词，需结合上下文与词典知识判定实体边界；避免将「北京大学」错切为「北京」「大学」。\n\
            - **中英文混合**：对「OpenAI 发布 GPT-5」「iPhone 17」等中英文混合实体，整体作为一个实体输出，保留原始大小写与符号。\n\
            - **繁简体**：保留原文繁简体形式，不做转换；「台積電」「台积电」均按原文字面输出。\n\
            - **数字与量词**：「999 元」「3 名」「2026 年」中的数字与单位按 NUMBER 处理；纯日期归 TIME。\n\
            - **时间表达**：支持绝对时间（2026-07-20）与相对时间（昨天、去年、本季度）。\n\
            - **嵌套实体**：若实体嵌套（如「北京冬奥会」含「北京」+「冬奥会」），仅输出最外层语义最完整的实体。";

        // 6. few-shot（10 个示例，覆盖 6 类实体）
        let few_shot = "## few-shot\n以下 10 个示例覆盖 6 类实体，作为输出格式与识别边界的参考：\n\
            \n示例 1.（人名）\n输入：{\"chunk\": \"张三在北京大学发表演讲\"}\n输出：{\"entities\": [{\"type\": \"PERSON\", \"text\": \"张三\", \"start\": 0, \"end\": 2}, {\"type\": \"ORGANIZATION\", \"text\": \"北京大学\", \"start\": 3, \"end\": 7}]}\n\
            \n示例 2.（地名）\n输入：{\"chunk\": \"上海外滩夜景迷人\"}\n输出：{\"entities\": [{\"type\": \"LOCATION\", \"text\": \"上海\", \"start\": 0, \"end\": 2}, {\"type\": \"LOCATION\", \"text\": \"外滩\", \"start\": 2, \"end\": 4}]}\n\
            \n示例 3.（机构）\n输入：{\"chunk\": \"阿里巴巴发布财报\"}\n输出：{\"entities\": [{\"type\": \"ORGANIZATION\", \"text\": \"阿里巴巴\", \"start\": 0, \"end\": 4}]}\n\
            \n示例 4.（时间）\n输入：{\"chunk\": \"会议定于 2026 年 7 月 20 日召开\"}\n输出：{\"entities\": [{\"type\": \"TIME\", \"text\": \"2026 年 7 月 20 日\", \"start\": 5, \"end\": 18}, {\"type\": \"EVENT\", \"text\": \"会议\", \"start\": 0, \"end\": 2}]}\n\
            \n示例 5.（数字）\n输入：{\"chunk\": \"售价 999 元，限量 1000 件\"}\n输出：{\"entities\": [{\"type\": \"NUMBER\", \"text\": \"999\", \"start\": 3, \"end\": 6}, {\"type\": \"NUMBER\", \"text\": \"1000\", \"start\": 12, \"end\": 16}]}\n\
            \n示例 6.（事件）\n输入：{\"chunk\": \"苹果公司发布 iPhone 17\"}\n输出：{\"entities\": [{\"type\": \"ORGANIZATION\", \"text\": \"苹果公司\", \"start\": 0, \"end\": 4}, {\"type\": \"EVENT\", \"text\": \"发布 iPhone 17\", \"start\": 4, \"end\": 17}]}\n\
            \n示例 7.（人名 + 机构）\n输入：{\"chunk\": \"李四加入腾讯\"}\n输出：{\"entities\": [{\"type\": \"PERSON\", \"text\": \"李四\", \"start\": 0, \"end\": 2}, {\"type\": \"ORGANIZATION\", \"text\": \"腾讯\", \"start\": 4, \"end\": 6}]}\n\
            \n示例 8.（地名 + 时间 + 事件）\n输入：{\"chunk\": \"去年北京冬奥会成功举办\"}\n输出：{\"entities\": [{\"type\": \"TIME\", \"text\": \"去年\", \"start\": 0, \"end\": 2}, {\"type\": \"LOCATION\", \"text\": \"北京\", \"start\": 2, \"end\": 4}, {\"type\": \"EVENT\", \"text\": \"冬奥会\", \"start\": 4, \"end\": 7}]}\n\
            \n示例 9.（数字 + 机构）\n输入：{\"chunk\": \"3 名腾讯员工获奖\"}\n输出：{\"entities\": [{\"type\": \"NUMBER\", \"text\": \"3\", \"start\": 0, \"end\": 1}, {\"type\": \"ORGANIZATION\", \"text\": \"腾讯\", \"start\": 2, \"end\": 4}]}\n\
            \n示例 10.（复合：时间 + 机构 + 事件）\n输入：{\"chunk\": \"2025 年 5 月 18 日，OpenAI 发布 GPT-5\"}\n输出：{\"entities\": [{\"type\": \"TIME\", \"text\": \"2025 年 5 月 18 日\", \"start\": 0, \"end\": 13}, {\"type\": \"ORGANIZATION\", \"text\": \"OpenAI\", \"start\": 14, \"end\": 20}, {\"type\": \"EVENT\", \"text\": \"发布 GPT-5\", \"start\": 21, \"end\": 30}]}";

        // 7. 约束
        let constraints = "## 约束\n\
            1. **仅输出 JSON**：响应必须是单一 JSON 对象，不得包含 markdown 代码块标记、解释文字、注释或前后缀。\n\
            2. **不输出解释**：不要在 JSON 之外提供任何自然语言说明。\n\
            3. **实体 text 必须是原文子串**：`text` 字段必须严格等于 `chunk` 中 `[start..end]` 范围的字符子串，不得改写、补全或翻译。\n\
            4. **偏移为字符级**：`start` / `end` 以 Unicode 标量值（字符）为单位，不是字节偏移。\n\
            5. **不重复**：同一实体在原文中多次出现时，每次出现均单独输出一条记录。\n\
            6. **不臆测**：仅识别文本中明确出现的实体，不基于常识补全。\n\
            7. **未识别时返回空数组**：若文本中无任何目标实体，返回 `{\"entities\": []}`。\n\
            8. **类型严格**：`type` 必须为 6 类枚举之一（PERSON / LOCATION / ORGANIZATION / TIME / NUMBER / EVENT），不得自创类型。";

        SevenSection {
            role: role.to_string(),
            task,
            input_format: input_format.to_string(),
            output_format: output_format.to_string(),
            chinese_adaptation: chinese_adaptation.to_string(),
            few_shot: few_shot.to_string(),
            constraints: constraints.to_string(),
        }
    }
}

impl PromptTemplate for NerPrompt {
    fn render(&self, context: &PromptContext) -> String {
        let sections = Self::build_sections(context);
        SevenSectionPrompt::build(sections, context)
    }
}
