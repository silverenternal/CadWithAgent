# CadAgent Web UI 开发指南

## 概述

CadAgent Web UI 是一个基于现代 Web 技术构建的交互式 CAD 设计界面，提供：

- **3D 可视化**: 使用 Three.js 和 React Three Fiber 进行实时 3D 渲染
- **AI 辅助设计**: 集成 LLM 推理引擎，支持自然语言设计命令
- **参数化建模**: Feature Tree 管理设计历史和参数
- **实时协作**: 基于 WebSocket 的多用户协作（规划中）

## 快速开始

### 1. 安装依赖

```bash
cd web-ui
npm install
```

### 2. 启动开发服务器

```bash
npm run dev
```

访问 http://localhost:3000

### 3. 启动后端 API 服务器

```bash
# 从项目根目录
cargo run -- serve

# 或者指定端口
cargo run -- serve --port 9000
```

## 架构设计

### 前端架构

```
web-ui/src/
├── components/          # React 组件
│   ├── App.tsx         # 主应用组件
│   ├── CADModel.tsx    # 3D 模型渲染
│   ├── ChatPanel.tsx   # AI 聊天界面
│   ├── FeatureTree.tsx # 特征树面板
│   ├── PropertiesPanel.tsx  # 属性编辑器
│   └── Toolbar.tsx     # 工具栏
├── hooks/
│   └── useStore.ts     # Zustand 状态管理
├── utils/
│   └── api.ts          # API 客户端
├── types.ts            # TypeScript 类型定义
└── styles/
    └── globals.css     # 全局样式
```

### 状态管理

使用 Zustand 进行全局状态管理：

```typescript
// 状态结构
interface AppState {
  primitives: Primitive[]      // 几何图元
  selectedIds: string[]        // 选中的图元 ID
  darkMode: boolean            // 深色模式
  chatMessages: ChatMessage[]  // 聊天历史
  isChatLoading: boolean       // 聊天加载状态
}
```

### 3D 渲染

使用 React Three Fiber 声明式 3D 渲染：

```tsx
<Canvas camera={{ position: [5, 5, 5], fov: 50 }}>
  <ambientLight intensity={0.5} />
  <directionalLight position={[10, 10, 5]} />
  <CADModel primitives={primitives} />
  <OrbitControls />
</Canvas>
```

## 扩展开发

### 添加新的图元类型

1. **定义类型** (`src/types.ts`):

```typescript
export interface Spline {
  type: 'spline'
  controlPoints: Point[]
  degree: number
  id?: string
}

export type Primitive = Line | Circle | Rectangle | Polygon | Arc | Spline
```

2. **实现渲染** (`src/components/CADModel.tsx`):

```tsx
case 'spline': {
  const points = primitive.controlPoints.map(p => 
    new THREE.Vector3(p.x, p.y, p.z || 0)
  )
  const curve = new THREE.CatmullRomCurve3(points)
  const geometry = new THREE.TubeGeometry(curve, 64, 0.05, 8, false)
  return <mesh key={id} geometry={geometry}>
    <meshStandardMaterial color={color} />
  </mesh>
}
```

3. **添加属性面板** (`src/components/PropertiesPanel.tsx`):

```tsx
case 'spline':
  return (
    <>
      <PropertyGroup title="Control Points">
        {primitive.controlPoints.map((point, i) => (
          <CoordinateInput key={i} {...point} />
        ))}
      </PropertyGroup>
      <PropertyGroup title="Degree">
        <NumberInput value={primitive.degree} />
      </PropertyGroup>
    </>
  )
```

### 添加新的工具

1. **在工具栏添加按钮** (`src/components/Toolbar.tsx`):

```tsx
const tools = [
  { id: 'select', label: 'Select', icon: '🖱️' },
  { id: 'spline', label: 'Spline', icon: '〜' },  // 新增
]
```

2. **实现工具逻辑**:

```tsx
// 在父组件中处理工具切换
const [activeTool, setActiveTool] = useState<string | null>(null)

const handleCanvasClick = (e) => {
  if (activeTool === 'spline') {
    // 添加样条曲线控制点
  }
}
```

### 集成后端 API

1. **定义 API 接口** (`src/utils/api.ts`):

```typescript
export async function createSpline(params: {
  controlPoints: Point[]
  degree: number
}): Promise<Primitive> {
  const response = await api.post('/tools/execute', {
    tool: 'create_spline',
    parameters: params,
  })
  return response.data
}
```

2. **在组件中使用**:

```tsx
const handleSubmit = async () => {
  try {
    const spline = await createSpline({
      controlPoints: points,
      degree: 3,
    })
    addPrimitive(spline)
  } catch (error) {
    console.error('Failed to create spline:', error)
  }
}
```

## 性能优化

### 1. 3D 渲染优化

- 使用 `React.memo` 避免不必要的重渲染
- 对大量图元使用 InstancedMesh
- 实现视锥体剔除

```tsx
const CADModel = React.memo(({ primitives }) => {
  // ...
})
```

### 2. 状态更新优化

使用 Zustand 的选择器避免过度更新：

```tsx
// 好的做法
const selectedIds = useStore(state => state.selectedIds)

// 避免：订阅整个 store
const store = useStore()
```

### 3. 代码分割

使用 React.lazy 和 Suspense：

```tsx
const PropertiesPanel = React.lazy(() => 
  import('./components/PropertiesPanel')
)

function App() {
  return (
    <Suspense fallback={<div>Loading...</div>}>
      <PropertiesPanel />
    </Suspense>
  )
}
```

## 测试

### 单元测试

```bash
npm run test
```

### E2E 测试

```bash
npm run test:e2e
```

## 构建和部署

### 生产构建

```bash
npm run build
```

输出在 `dist/` 目录

### 部署到 Nginx

```nginx
server {
    listen 80;
    server_name cadagent.example.com;
    root /var/www/cadagent;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}
```

### Docker 部署

```dockerfile
FROM node:18-alpine as build
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=build /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
```

## 调试技巧

### React DevTools

安装 React DevTools 扩展检查组件树和状态

### Three.js Inspector

使用 Spector.js 调试 WebGL 调用

### 网络请求

使用浏览器开发者工具的 Network 面板监控 API 请求

## 常见问题

### Q: 3D 模型显示模糊？

A: 检查 Canvas 的 DPR 设置：
```tsx
<Canvas dpr={[1, 2]}>  // 限制最大像素比为 2
```

### Q: 大量图元性能下降？

A: 使用 LOD (Level of Detail) 和 InstancedMesh

### Q: 聊天响应慢？

A: 实现流式响应和乐观更新

## 参考资源

- [React Three Fiber 文档](https://docs.pmnd.rs/react-three-fiber)
- [Three.js 文档](https://threejs.org/docs/)
- [Zustand 文档](https://github.com/pmndrs/zustand)
- [Tailwind CSS 文档](https://tailwindcss.com/docs)
- [shadcn/ui 组件](https://ui.shadcn.com)

## 贡献

欢迎提交 Issue 和 Pull Request！
