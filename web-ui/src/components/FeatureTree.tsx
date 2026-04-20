import { useState } from 'react'
import { 
  ChevronRight, 
  ChevronDown, 
  Eye, 
  EyeOff, 
  Folder, 
  FileBox,
  Plus,
  MoreHorizontal
} from 'lucide-react'
import { useStore } from '../hooks/useStore'
import type { Feature } from '../types'

// Sample features for demonstration
const sampleFeatures: Feature[] = [
  {
    id: '1',
    name: 'Base Sketch',
    type: 'sketch',
    visible: true,
    suppressed: false,
  },
  {
    id: '2',
    name: 'Extrude 1',
    type: 'extrude',
    parentId: '1',
    visible: true,
    suppressed: false,
  },
  {
    id: '3',
    name: 'Fillet 1',
    type: 'fillet',
    parentId: '2',
    visible: true,
    suppressed: false,
  },
  {
    id: '4',
    name: 'Cut Extrude 1',
    type: 'cut',
    parentId: '2',
    visible: true,
    suppressed: false,
  },
]

export function FeatureTree() {
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set(['1', '2']))
  const [features] = useState<Feature[]>(sampleFeatures)
  
  const toggleExpanded = (id: string) => {
    const newExpanded = new Set(expandedIds)
    if (newExpanded.has(id)) {
      newExpanded.delete(id)
    } else {
      newExpanded.add(id)
    }
    setExpandedIds(newExpanded)
  }
  
  const getIcon = (type: Feature['type']) => {
    switch (type) {
      case 'sketch':
        return <FileBox size={16} className="text-yellow-500" />
      case 'extrude':
        return <Folder size={16} className="text-blue-500" />
      case 'cut':
        return <Folder size={16} className="text-red-500" />
      case 'fillet':
        return <Folder size={16} className="text-green-500" />
      case 'chamfer':
        return <Folder size={16} className="text-purple-500" />
      case 'pattern':
        return <Folder size={16} className="text-orange-500" />
      default:
        return <FileBox size={16} />
    }
  }
  
  const renderFeature = (feature: Feature, depth = 0) => {
    const hasChildren = features.some(f => f.parentId === feature.id)
    const isExpanded = expandedIds.has(feature.id)
    
    return (
      <div key={feature.id}>
        <div
          className="flex items-center gap-1 px-2 py-1.5 hover:bg-accent cursor-pointer group"
          style={{ paddingLeft: `${depth * 16 + 8}px` }}
          onClick={() => hasChildren && toggleExpanded(feature.id)}
        >
          {hasChildren ? (
            isExpanded ? (
              <ChevronDown size={16} className="text-muted-foreground" />
            ) : (
              <ChevronRight size={16} className="text-muted-foreground" />
            )
          ) : (
            <div className="w-4" />
          )}
          
          {getIcon(feature.type)}
          
          <span className="text-sm flex-1 truncate">{feature.name}</span>
          
          <button
            className="opacity-0 group-hover:opacity-100 p-1 hover:bg-accent rounded"
            title={feature.visible ? 'Hide' : 'Show'}
          >
            {feature.visible ? (
              <Eye size={14} className="text-muted-foreground" />
            ) : (
              <EyeOff size={14} className="text-muted-foreground" />
            )}
          </button>
          
          <button
            className="opacity-0 group-hover:opacity-100 p-1 hover:bg-accent rounded"
            title="More options"
          >
            <MoreHorizontal size={14} className="text-muted-foreground" />
          </button>
        </div>
        
        {isExpanded && hasChildren && (
          <div>
            {features
              .filter(f => f.parentId === feature.id)
              .map(child => renderFeature(child, depth + 1))}
          </div>
        )}
      </div>
    )
  }
  
  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-3 border-b flex items-center justify-between">
        <h3 className="font-semibold">Feature Tree</h3>
        <button className="p-1 rounded hover:bg-accent" title="Add feature">
          <Plus size={18} />
        </button>
      </div>
      
      {/* Tree */}
      <div className="flex-1 overflow-y-auto">
        {features
          .filter(f => !f.parentId)
          .map(feature => renderFeature(feature))}
      </div>
    </div>
  )
}
