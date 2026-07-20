//! Sub-Step 10.3.1 — 事件提取 7 段式 prompt 模板（D2.15 决策）
//!
//! ## 任务
//! 从给定中文文本块中提取事件，生成 EventCandidate 结构（含 title / summary /
//! content / category / keywords / entities）。
//!
//! ## 7 段结构
//! 1. 角色：你是中文事件提取专家
//! 2. 任务：识别事件并产出结构化 EventCandidate
//! 3. 输入格式：JSON `{"chunk": "..."}`
//! 4. 输出格式：JSON `{title, summary, content, category, keywords, entities}`
//! 5. 中文适配：分词边界 / 中英混合 / 繁简体
//! 6. few-shot：10 个中文示例
//! 7. 约束：仅输出 JSON / 不输出解释 / 字段必填

use super::{PromptContext, PromptTemplate, SevenSection, SevenSectionPrompt};

/// 事件提取 prompt 模板
#[derive(Debug, Clone, Default)]
pub struct ExtractPrompt;

impl ExtractPrompt {
    /// 创建一个新的事件提取 prompt 模板
    pub fn new() -> Self {
        Self
    }

    /// 构造 7 段内容（不含 chunk 替换，由 [`SevenSectionPrompt::build`] 完成）
    fn build_sections(context: &PromptContext) -> SevenSection {
        // 1. 角色
        let role = "## 角色\n你是一位中文事件提取专家，擅长从新闻、文档、对话片段中识别事件的触发、参与者、时间、地点，并将其结构化为可入库的 EventCandidate 对象。";

        // 2. 任务
        let task = format!(
            "## 任务\n从用户提供的文本块中识别一个或多个事件，并产出结构化 EventCandidate。每个 EventCandidate 应包含：\n\
             - `title`：事件标题（简短一句话，不超过 30 字）\n\
             - `summary`：事件摘要（1-3 句话，不超过 100 字）\n\
             - `content`：事件正文（保留原文关键信息，可适当压缩冗余）\n\
             - `category`：事件类别（如「科技」「财经」「体育」「政治」「社会」「教育」「医疗」「其他」）\n\
             - `keywords`：3-5 个关键词列表（用于检索与聚类）\n\
             - `entities`：关联实体列表（含 PERSON / LOCATION / ORGANIZATION / TIME / NUMBER / EVENT 等）\n\
             \n本次需识别的实体类型范围：[{}]",
            context.entity_types.join(", ")
        );

        // 3. 输入格式（含 {chunk} 占位符）
        let input_format = "## 输入格式\n输入为 JSON 对象：\n\
            ```json\n\
            {\"chunk\": \"待提取事件的中文文本块\"}\n\
            ```\n\
            \n本次输入：\n\
            ```json\n\
            {\"chunk\": \"{chunk}\"}\n\
            ```";

        // 4. 输出格式
        let output_format = "## 输出格式\n输出严格遵循以下 JSON Schema：\n\
            ```json\n\
            {\n  \
              \"events\": [\n    \
                {\n      \
                  \"title\": \"OpenAI 发布 GPT-5\",\n      \
                  \"summary\": \"2025 年 5 月 18 日，OpenAI 正式发布 GPT-5 模型。\",\n      \
                  \"content\": \"2025 年 5 月 18 日，OpenAI 发布 GPT-5...（保留原文关键信息）\",\n      \
                  \"category\": \"科技\",\n      \
                  \"keywords\": [\"OpenAI\", \"GPT-5\", \"发布\"],\n      \
                  \"entities\": [\n        \
                    {\"type\": \"TIME\", \"text\": \"2025 年 5 月 18 日\", \"start\": 0, \"end\": 13},\n        \
                    {\"type\": \"ORGANIZATION\", \"text\": \"OpenAI\", \"start\": 14, \"end\": 20},\n        \
                    {\"type\": \"EVENT\", \"text\": \"发布 GPT-5\", \"start\": 21, \"end\": 30}\n      \
                  ]\n    \
                }\n  \
              ]\n\
            }\n\
            ```\n\
            字段说明：\n\
            - `title`：简明扼要的事件标题，避免主观色彩\n\
            - `summary`：高度浓缩的事件摘要，包含 5W1H 关键要素\n\
            - `content`：事件正文，可引用原文核心句子\n\
            - `category`：单一类别字符串（8 类之一）\n\
            - `keywords`：3-5 个关键词数组\n\
            - `entities`：事件关联实体数组（字段与 NER 输出一致）";

        // 5. 中文适配
        let chinese_adaptation = "## 中文适配\n\
            - **事件触发词识别**：中文事件常由「发布」「召开」「成立」「签订」「爆发」「完成」等动词触发，需结合上下文判定事件边界。\n\
            - **多事件文本**：一个 chunk 可能包含多个事件（如「A 公司发布产品，同时 B 公司宣布收购」），需拆分为多个 EventCandidate。\n\
            - **中英文混合**：实体名「OpenAI」「iPhone 17」整体保留，不做翻译或音译。\n\
            - **繁简体**：保留原文形式，不做转换。\n\
            - **时间归一化**：相对时间（昨天、上季度）保留原文形式入库，归一化由下游处理。\n\
            - **隐式参与者**：若文本中未明确出现但可推断的主语（如「公司宣布」中的「公司」），按原文形式提取，不补全。\n\
            - **类别判定**：依据事件主体与动作综合判定 category，跨领域时取最主要的一个。";

        // 6. few-shot（10 个示例，覆盖科技/财经/体育/政治/社会/教育/医疗）
        let few_shot = "## few-shot\n以下 10 个示例覆盖科技 / 财经 / 体育 / 政治 / 社会 / 教育 / 医疗 等类别：\n\
            \n示例 1.（科技 - 产品发布）\n输入：{\"chunk\": \"苹果公司发布 iPhone 17\"}\n输出：{\"events\": [{\"title\": \"苹果公司发布 iPhone 17\", \"summary\": \"苹果公司正式发布 iPhone 17。\", \"content\": \"苹果公司发布 iPhone 17\", \"category\": \"科技\", \"keywords\": [\"苹果公司\", \"iPhone 17\", \"发布\"], \"entities\": [{\"type\": \"ORGANIZATION\", \"text\": \"苹果公司\", \"start\": 0, \"end\": 4}, {\"type\": \"EVENT\", \"text\": \"发布 iPhone 17\", \"start\": 4, \"end\": 17}]}]}\n\
            \n示例 2.（科技 - 模型发布）\n输入：{\"chunk\": \"2025 年 5 月 18 日，OpenAI 发布 GPT-5\"}\n输出：{\"events\": [{\"title\": \"OpenAI 发布 GPT-5\", \"summary\": \"2025 年 5 月 18 日，OpenAI 正式发布 GPT-5 模型。\", \"content\": \"2025 年 5 月 18 日，OpenAI 发布 GPT-5\", \"category\": \"科技\", \"keywords\": [\"OpenAI\", \"GPT-5\", \"发布\", \"2025\"], \"entities\": [{\"type\": \"TIME\", \"text\": \"2025 年 5 月 18 日\", \"start\": 0, \"end\": 13}, {\"type\": \"ORGANIZATION\", \"text\": \"OpenAI\", \"start\": 14, \"end\": 20}, {\"type\": \"EVENT\", \"text\": \"发布 GPT-5\", \"start\": 21, \"end\": 30}]}]}\n\
            \n示例 3.（财经 - 财报发布）\n输入：{\"chunk\": \"阿里巴巴发布 2026 财年第一季度财报\"}\n输出：{\"events\": [{\"title\": \"阿里巴巴发布 Q1 财报\", \"summary\": \"阿里巴巴发布 2026 财年第一季度财报。\", \"content\": \"阿里巴巴发布 2026 财年第一季度财报\", \"category\": \"财经\", \"keywords\": [\"阿里巴巴\", \"财报\", \"2026 财年\"], \"entities\": [{\"type\": \"ORGANIZATION\", \"text\": \"阿里巴巴\", \"start\": 0, \"end\": 4}, {\"type\": \"TIME\", \"text\": \"2026 财年第一季度\", \"start\": 6, \"end\": 17}, {\"type\": \"EVENT\", \"text\": \"发布\", \"start\": 4, \"end\": 6}]}]}\n\
            \n示例 4.（体育 - 赛事举办）\n输入：{\"chunk\": \"去年北京冬奥会成功举办\"}\n输出：{\"events\": [{\"title\": \"北京冬奥会成功举办\", \"summary\": \"去年北京冬奥会成功举办。\", \"content\": \"去年北京冬奥会成功举办\", \"category\": \"体育\", \"keywords\": [\"北京冬奥会\", \"去年\", \"举办\"], \"entities\": [{\"type\": \"TIME\", \"text\": \"去年\", \"start\": 0, \"end\": 2}, {\"type\": \"LOCATION\", \"text\": \"北京\", \"start\": 2, \"end\": 4}, {\"type\": \"EVENT\", \"text\": \"冬奥会\", \"start\": 4, \"end\": 7}]}]}\n\
            \n示例 5.（社会 - 人事变动）\n输入：{\"chunk\": \"李四加入腾讯\"}\n输出：{\"events\": [{\"title\": \"李四加入腾讯\", \"summary\": \"李四加入腾讯公司。\", \"content\": \"李四加入腾讯\", \"category\": \"社会\", \"keywords\": [\"李四\", \"腾讯\", \"加入\"], \"entities\": [{\"type\": \"PERSON\", \"text\": \"李四\", \"start\": 0, \"end\": 2}, {\"type\": \"ORGANIZATION\", \"text\": \"腾讯\", \"start\": 4, \"end\": 6}, {\"type\": \"EVENT\", \"text\": \"加入\", \"start\": 2, \"end\": 4}]}]}\n\
            \n示例 6.（社会 - 演讲活动）\n输入：{\"chunk\": \"张三在北京大学发表演讲\"}\n输出：{\"events\": [{\"title\": \"张三在北京大学发表演讲\", \"summary\": \"张三在北京大学发表演讲。\", \"content\": \"张三在北京大学发表演讲\", \"category\": \"社会\", \"keywords\": [\"张三\", \"北京大学\", \"演讲\"], \"entities\": [{\"type\": \"PERSON\", \"text\": \"张三\", \"start\": 0, \"end\": 2}, {\"type\": \"LOCATION\", \"text\": \"北京\", \"start\": 3, \"end\": 5}, {\"type\": \"ORGANIZATION\", \"text\": \"北京大学\", \"start\": 3, \"end\": 7}, {\"type\": \"EVENT\", \"text\": \"发表演讲\", \"start\": 7, \"end\": 11}]}]}\n\
            \n示例 7.（教育 - 学术活动）\n输入：{\"chunk\": \"会议定于 2026 年 7 月 20 日召开\"}\n输出：{\"events\": [{\"title\": \"会议将于 2026 年 7 月 20 日召开\", \"summary\": \"会议定于 2026 年 7 月 20 日召开。\", \"content\": \"会议定于 2026 年 7 月 20 日召开\", \"category\": \"教育\", \"keywords\": [\"会议\", \"2026 年 7 月 20 日\", \"召开\"], \"entities\": [{\"type\": \"EVENT\", \"text\": \"会议\", \"start\": 0, \"end\": 2}, {\"type\": \"TIME\", \"text\": \"2026 年 7 月 20 日\", \"start\": 5, \"end\": 18}]}]}\n\
            \n示例 8.（社会 - 颁奖）\n输入：{\"chunk\": \"3 名腾讯员工获奖\"}\n输出：{\"events\": [{\"title\": \"3 名腾讯员工获奖\", \"summary\": \"3 名腾讯员工获奖。\", \"content\": \"3 名腾讯员工获奖\", \"category\": \"社会\", \"keywords\": [\"腾讯\", \"员工\", \"获奖\"], \"entities\": [{\"type\": \"NUMBER\", \"text\": \"3\", \"start\": 0, \"end\": 1}, {\"type\": \"ORGANIZATION\", \"text\": \"腾讯\", \"start\": 2, \"end\": 4}, {\"type\": \"EVENT\", \"text\": \"获奖\", \"start\": 6, \"end\": 8}]}]}\n\
            \n示例 9.（医疗 - 健康事件）\n输入：{\"chunk\": \"上海发现 5 例新型流感病例\"}\n输出：{\"events\": [{\"title\": \"上海发现新型流感病例\", \"summary\": \"上海发现 5 例新型流感病例。\", \"content\": \"上海发现 5 例新型流感病例\", \"category\": \"医疗\", \"keywords\": [\"上海\", \"流感\", \"病例\", \"发现\"], \"entities\": [{\"type\": \"LOCATION\", \"text\": \"上海\", \"start\": 0, \"end\": 2}, {\"type\": \"EVENT\", \"text\": \"发现\", \"start\": 2, \"end\": 4}, {\"type\": \"NUMBER\", \"text\": \"5\", \"start\": 4, \"end\": 5}, {\"type\": \"DISEASE\", \"text\": \"新型流感\", \"start\": 6, \"end\": 10}]}]}\n\
            \n示例 10.（政治 - 外事活动）\n输入：{\"chunk\": \"中国与俄罗斯签署能源合作协议\"}\n输出：{\"events\": [{\"title\": \"中俄签署能源合作协议\", \"summary\": \"中国与俄罗斯签署能源合作协议。\", \"content\": \"中国与俄罗斯签署能源合作协议\", \"category\": \"政治\", \"keywords\": [\"中国\", \"俄罗斯\", \"能源合作\", \"协议\"], \"entities\": [{\"type\": \"LOCATION\", \"text\": \"中国\", \"start\": 0, \"end\": 2}, {\"type\": \"LOCATION\", \"text\": \"俄罗斯\", \"start\": 3, \"end\": 6}, {\"type\": \"EVENT\", \"text\": \"签署\", \"start\": 6, \"end\": 8}, {\"type\": \"CONCEPT\", \"text\": \"能源合作协议\", \"start\": 8, \"end\": 14}]}]}";

        // 7. 约束
        let constraints = "## 约束\n\
            1. **仅输出 JSON**：响应必须是单一 JSON 对象，不得包含 markdown 代码块标记、解释文字、注释或前后缀。\n\
            2. **不输出解释**：不要在 JSON 之外提供任何自然语言说明。\n\
            3. **字段必填**：`title` / `summary` / `content` / `category` / `keywords` / `entities` 均不可省略；`keywords` 至少 3 项；`entities` 可为空数组。\n\
            4. **实体 text 必须是原文子串**：`entities[].text` 必须严格等于 `chunk` 中 `[start..end]` 范围的字符子串。\n\
            5. **偏移为字符级**：`start` / `end` 以 Unicode 标量值（字符）为单位。\n\
            6. **category 限定 8 类**：必须为「科技 / 财经 / 体育 / 政治 / 社会 / 教育 / 医疗 / 其他」之一。\n\
            7. **多事件拆分**：若 chunk 中包含多个独立事件，输出多个 EventCandidate，按出现顺序排列。\n\
            8. **未识别时返回空数组**：若文本中无明确事件，返回 `{\"events\": []}`。\n\
            9. **不臆测**：仅基于原文事实提取，不得引入外部知识或主观推测。\n\
            10. **类型严格**：`entities[].type` 必须为 11 类枚举之一（PERSON / LOCATION / ORGANIZATION / TIME / NUMBER / EVENT / OBJECT / CONCEPT / LAW / DISEASE / OTHER）。";

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

impl PromptTemplate for ExtractPrompt {
    fn render(&self, context: &PromptContext) -> String {
        let sections = Self::build_sections(context);
        SevenSectionPrompt::build(sections, context)
    }
}
