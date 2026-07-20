/**
 * SparkFox 路由配置 — createBrowserRouter
 *
 * 来源：SparkFox 全新设计
 * 功能：React Router 7 data mode 路由器
 *
 * 已落地（v0.1 落地，v0.2 验证通过）
 */

import { createBrowserRouter, Navigate } from 'react-router-dom';
import { sparkfoxRoutes } from './routes';

export const sparkfoxRouter = createBrowserRouter([
  {
    path: '/',
    element: <Navigate to="/" replace />,
  },
  ...sparkfoxRoutes.map((route) => ({
    path: route.path,
    element: <route.element />,
  })),
]);
