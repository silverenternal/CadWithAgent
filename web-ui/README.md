# CadAgent Web UI

A modern, AI-powered web interface for CadAgent - bringing professional CAD design capabilities to your browser.

## Features

### 🎨 3D Viewer
- Interactive 3D visualization using Three.js and React Three Fiber
- Orbit, pan, and zoom controls
- Grid and axis helpers
- Real-time primitive rendering (lines, circles, rectangles, polygons, arcs)
- Selection highlighting

### 🤖 AI Assistant
- Natural language CAD commands
- Conversational design interface
- Markdown support with syntax highlighting
- Multi-turn conversation context
- Action suggestions and recommendations

### 🌳 Feature Tree
- Parametric feature management
- Hierarchical organization
- Visibility toggles
- Support for sketches, extrusions, cuts, fillets, chamfers, and patterns

### 📋 Properties Panel
- Edit primitive properties
- Real-time value updates
- Coordinate inputs
- Geometric measurements (length, area, circumference)

### 🛠️ Toolbar
- Drawing tools (line, circle, rectangle, arc, polygon)
- File operations (open, save, export)
- Edit operations (undo, redo, delete)
- View toggles (chat, features, properties)
- Dark/light theme switching

## Tech Stack

- **Frontend Framework**: React 18 with TypeScript
- **Build Tool**: Vite
- **3D Rendering**: Three.js + React Three Fiber + Drei
- **Styling**: Tailwind CSS with shadcn/ui components
- **State Management**: Zustand
- **HTTP Client**: Axios
- **Markdown**: React Markdown with remark-gfm

## Getting Started

### Prerequisites

- Node.js 18+ 
- npm or yarn
- CadAgent backend running (optional, for API integration)

### Installation

```bash
# Navigate to web-ui directory
cd web-ui

# Install dependencies
npm install

# Start development server
npm run dev
```

The application will open at `http://localhost:3000`

### Build for Production

```bash
# Build optimized production bundle
npm run build

# Preview production build
npm run preview
```

## Configuration

### Environment Variables

Create a `.env` file in the root directory:

```env
VITE_API_URL=http://localhost:8080/api
```

### Backend Integration

The Web UI is designed to work with the CadAgent Rust backend via a REST API. The backend should expose the following endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/chat` | POST | Send message to AI assistant |
| `/api/upload` | POST | Upload CAD file |
| `/api/export/{format}` | POST | Export model |
| `/api/tools` | GET | List available tools |
| `/api/tools/execute` | POST | Execute tool |
| `/api/constraints/apply` | POST | Apply constraints |
| `/api/constraints/solve` | POST | Solve constraints |
| `/api/health` | GET | Health check |

## Project Structure

```
web-ui/
├── src/
│   ├── components/       # React components
│   │   ├── App.tsx
│   │   ├── CADModel.tsx         # 3D model renderer
│   │   ├── ChatPanel.tsx        # AI chat interface
│   │   ├── FeatureTree.tsx      # Feature tree panel
│   │   ├── PropertiesPanel.tsx  # Properties editor
│   │   └── Toolbar.tsx          # Top toolbar
│   ├── hooks/
│   │   └── useStore.ts          # Zustand state management
│   ├── utils/
│   │   └── api.ts               # API client
│   ├── styles/
│   │   └── globals.css          # Global styles
│   ├── types.ts                 # TypeScript types
│   └── main.tsx                 # Entry point
├── index.html
├── package.json
├── tailwind.config.js
├── tsconfig.json
└── vite.config.ts
```

## Development

### Code Style

```bash
# Lint code
npm run lint

# Fix lint issues
npm run lint:fix
```

### Adding New Primitives

1. Add the primitive type to `src/types.ts`
2. Implement rendering logic in `src/components/CADModel.tsx`
3. Add properties panel support in `src/components/PropertiesPanel.tsx`

### Adding New Tools

1. Define tool parameters in the backend
2. Update the toolbar in `src/components/Toolbar.tsx`
3. Implement tool execution via the API client

## Roadmap

- [ ] Real-time collaboration
- [ ] Version history
- [ ] Plugin system
- [ ] Mobile responsive design
- [ ] Offline support with PWA
- [ ] AR/VR visualization
- [ ] Advanced constraint editing
- [ ] Assembly modeling

## License

MIT License - see the main CadAgent repository for details.

## Contributing

Contributions are welcome! Please read the main [CONTRIBUTING.md](../CONTRIBUTING.md) file for guidelines.
