import { useState } from 'react'
import { 
  Menu, 
  Play, 
  Save, 
  Upload, 
  Download, 
  Settings, 
  Moon, 
  Sun,
  MessageSquare,
  TreeDeciduous,
  Sliders,
  Trash2,
  Undo,
  Redo
} from 'lucide-react'
import { useStore } from '../hooks/useStore'

interface ToolbarProps {
  onToggleChat: () => void
  onToggleFeatureTree: () => void
  onToggleProperties: () => void
}

export function Toolbar({ onToggleChat, onToggleFeatureTree, onToggleProperties }: ToolbarProps) {
  const { darkMode, toggleDarkMode } = useStore()
  const [activeTool, setActiveTool] = useState<string | null>(null)
  
  const tools = [
    { id: 'select', label: 'Select', icon: '🖱️' },
    { id: 'line', label: 'Line', icon: '📏' },
    { id: 'circle', label: 'Circle', icon: '⭕' },
    { id: 'rectangle', label: 'Rectangle', icon: '□' },
    { id: 'arc', label: 'Arc', icon: '◠' },
    { id: 'polygon', label: 'Polygon', icon: '⬡' },
  ]
  
  return (
    <header className="h-14 border-b bg-card flex items-center px-4 gap-4">
      {/* Logo */}
      <div className="flex items-center gap-2">
        <div className="w-8 h-8 bg-primary rounded-md flex items-center justify-center">
          <span className="text-primary-foreground font-bold">CA</span>
        </div>
        <span className="font-semibold text-lg">CadAgent</span>
      </div>
      
      {/* Divider */}
      <div className="w-px h-6 bg-border" />
      
      {/* File operations */}
      <div className="flex items-center gap-1">
        <ToolbarButton icon={<Upload size={18} />} label="Open" />
        <ToolbarButton icon={<Save size={18} />} label="Save" />
        <ToolbarButton icon={<Download size={18} />} label="Export" />
      </div>
      
      {/* Divider */}
      <div className="w-px h-6 bg-border" />
      
      {/* Edit operations */}
      <div className="flex items-center gap-1">
        <ToolbarButton icon={<Undo size={18} />} label="Undo" />
        <ToolbarButton icon={<Redo size={18} />} label="Redo" />
        <ToolbarButton icon={<Trash2 size={18} />} label="Delete" />
      </div>
      
      {/* Divider */}
      <div className="w-px h-6 bg-border" />
      
      {/* Tools */}
      <div className="flex items-center gap-1">
        {tools.map(tool => (
          <button
            key={tool.id}
            className={`px-3 py-1.5 rounded-md text-sm flex items-center gap-2 transition-colors
              ${activeTool === tool.id 
                ? 'bg-primary text-primary-foreground' 
                : 'hover:bg-accent hover:text-accent-foreground'
              }`}
            onClick={() => setActiveTool(tool.id === activeTool ? null : tool.id)}
            title={tool.label}
          >
            <span>{tool.icon}</span>
            <span className="hidden lg:inline">{tool.label}</span>
          </button>
        ))}
      </div>
      
      {/* Spacer */}
      <div className="flex-1" />
      
      {/* View toggles */}
      <div className="flex items-center gap-1">
        <ToolbarButton 
          icon={<MessageSquare size={18} />} 
          label="Chat" 
          onClick={onToggleChat}
        />
        <ToolbarButton 
          icon={<TreeDeciduous size={18} />} 
          label="Features" 
          onClick={onToggleFeatureTree}
        />
        <ToolbarButton 
          icon={<Sliders size={18} />} 
          label="Properties" 
          onClick={onToggleProperties}
        />
      </div>
      
      {/* Divider */}
      <div className="w-px h-6 bg-border" />
      
      {/* Theme toggle */}
      <button
        onClick={toggleDarkMode}
        className="p-2 rounded-md hover:bg-accent hover:text-accent-foreground transition-colors"
        title={darkMode ? 'Light mode' : 'Dark mode'}
      >
        {darkMode ? <Sun size={18} /> : <Moon size={18} />}
      </button>
      
      {/* Settings */}
      <button
        className="p-2 rounded-md hover:bg-accent hover:text-accent-foreground transition-colors"
        title="Settings"
      >
        <Settings size={18} />
      </button>
    </header>
  )
}

interface ToolbarButtonProps {
  icon: React.ReactNode
  label: string
  onClick?: () => void
}

function ToolbarButton({ icon, label, onClick }: ToolbarButtonProps) {
  return (
    <button
      onClick={onClick}
      className="p-2 rounded-md hover:bg-accent hover:text-accent-foreground transition-colors flex items-center gap-2"
      title={label}
    >
      {icon}
      <span className="hidden lg:inline text-sm">{label}</span>
    </button>
  )
}
