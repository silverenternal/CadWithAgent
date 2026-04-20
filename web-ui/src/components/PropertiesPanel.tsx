import { useState } from 'react'
import { useStore } from '../hooks/useStore'

export function PropertiesPanel() {
  const { selectedIds, primitives } = useStore()
  const selectedPrimitives = primitives.filter(p => selectedIds.includes(p.id || ''))
  
  if (selectedIds.length === 0) {
    return (
      <div className="h-full p-4">
        <h3 className="font-semibold mb-4">Properties</h3>
        <div className="text-sm text-muted-foreground text-center py-8">
          Select a primitive to view its properties
        </div>
      </div>
    )
  }
  
  return (
    <div className="h-full overflow-y-auto">
      <div className="p-3 border-b">
        <h3 className="font-semibold">Properties</h3>
        <p className="text-xs text-muted-foreground">
          {selectedIds.length} item(s) selected
        </p>
      </div>
      
      <div className="p-3 space-y-4">
        {selectedPrimitives.map((primitive, index) => (
          <PrimitiveProperties 
            key={primitive.id || index} 
            primitive={primitive} 
          />
        ))}
      </div>
    </div>
  )
}

import type { Primitive } from '../types'

interface PrimitivePropertiesProps {
  primitive: Primitive
}

function PrimitiveProperties({ primitive }: PrimitivePropertiesProps) {
  const formatNumber = (n: number) => n.toFixed(2)
  
  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2">
        <span className="text-xs font-medium px-2 py-1 bg-primary/10 text-primary rounded">
          {primitive.type.toUpperCase()}
        </span>
        <span className="text-xs text-muted-foreground">
          ID: {primitive.id || 'N/A'}
        </span>
      </div>
      
      {primitive.type === 'line' && (
        <>
          <PropertyGroup title="Start Point">
            <CoordinateInput x={primitive.start.x} y={primitive.start.y} z={primitive.start.z} />
          </PropertyGroup>
          <PropertyGroup title="End Point">
            <CoordinateInput x={primitive.end.x} y={primitive.end.y} z={primitive.end.z} />
          </PropertyGroup>
          <PropertyGroup title="Length">
            <div className="text-sm font-mono">
              {formatNumber(Math.sqrt(
                Math.pow(primitive.end.x - primitive.start.x, 2) +
                Math.pow(primitive.end.y - primitive.start.y, 2)
              ))}
            </div>
          </PropertyGroup>
        </>
      )}
      
      {primitive.type === 'circle' && (
        <>
          <PropertyGroup title="Center">
            <CoordinateInput x={primitive.center.x} y={primitive.center.y} z={primitive.center.z} />
          </PropertyGroup>
          <PropertyGroup title="Radius">
            <NumberInput value={primitive.radius} />
          </PropertyGroup>
          <PropertyGroup title="Diameter">
            <div className="text-sm font-mono">
              {formatNumber(primitive.radius * 2)}
            </div>
          </PropertyGroup>
          <PropertyGroup title="Circumference">
            <div className="text-sm font-mono">
              {formatNumber(2 * Math.PI * primitive.radius)}
            </div>
          </PropertyGroup>
        </>
      )}
      
      {primitive.type === 'rectangle' && (
        <>
          <PropertyGroup title="Origin">
            <CoordinateInput x={primitive.origin.x} y={primitive.origin.y} z={primitive.origin.z} />
          </PropertyGroup>
          <PropertyGroup title="Width">
            <NumberInput value={primitive.width} />
          </PropertyGroup>
          <PropertyGroup title="Height">
            <NumberInput value={primitive.height} />
          </PropertyGroup>
          <PropertyGroup title="Area">
            <div className="text-sm font-mono">
              {formatNumber(primitive.width * primitive.height)}
            </div>
          </PropertyGroup>
        </>
      )}
      
      {primitive.type === 'polygon' && (
        <>
          <PropertyGroup title="Vertices">
            <div className="text-sm font-mono">
              {primitive.points.length} points
            </div>
          </PropertyGroup>
          <div className="space-y-1">
            {primitive.points.map((point, i) => (
              <div key={i} className="text-xs">
                <span className="text-muted-foreground">P{i}:</span>{' '}
                <span className="font-mono">
                  ({formatNumber(point.x)}, {formatNumber(point.y)})
                </span>
              </div>
            ))}
          </div>
        </>
      )}
      
      {primitive.type === 'arc' && (
        <>
          <PropertyGroup title="Center">
            <CoordinateInput x={primitive.center.x} y={primitive.center.y} z={primitive.center.z} />
          </PropertyGroup>
          <PropertyGroup title="Radius">
            <NumberInput value={primitive.radius} />
          </PropertyGroup>
          <PropertyGroup title="Start Angle">
            <NumberInput value={primitive.startAngle} />
          </PropertyGroup>
          <PropertyGroup title="End Angle">
            <NumberInput value={primitive.endAngle} />
          </PropertyGroup>
        </>
      )}
    </div>
  )
}

interface PropertyGroupProps {
  title: string
  children: React.ReactNode
}

function PropertyGroup({ title, children }: PropertyGroupProps) {
  return (
    <div>
      <label className="text-xs text-muted-foreground mb-1 block">{title}</label>
      {children}
    </div>
  )
}

interface CoordinateInputProps {
  x: number
  y: number
  z?: number
}

function CoordinateInput({ x, y, z }: CoordinateInputProps) {
  return (
    <div className="flex gap-1">
      <CoordinateField label="X" value={x} />
      <CoordinateField label="Y" value={y} />
      {z !== undefined && <CoordinateField label="Z" value={z} />}
    </div>
  )
}

interface CoordinateFieldProps {
  label: string
  value: number
}

function CoordinateField({ label, value }: CoordinateFieldProps) {
  const [val, setVal] = useState(value.toFixed(2))
  
  return (
    <div className="flex-1">
      <label className="text-xs text-muted-foreground mb-0.5 block">{label}</label>
      <input
        type="text"
        value={val}
        onChange={(e) => setVal(e.target.value)}
        className="w-full px-2 py-1 bg-background border rounded text-sm font-mono focus:outline-none focus:ring-2 focus:ring-primary"
      />
    </div>
  )
}

interface NumberInputProps {
  value: number
}

function NumberInput({ value }: NumberInputProps) {
  const [val, setVal] = useState(value.toFixed(2))
  
  return (
    <input
      type="text"
      value={val}
      onChange={(e) => setVal(e.target.value)}
      className="w-full px-2 py-1 bg-background border rounded text-sm font-mono focus:outline-none focus:ring-2 focus:ring-primary"
    />
  )
}
