export interface Point {
  x: number
  y: number
  z?: number
}

export interface Line {
  type: 'line'
  start: Point
  end: Point
  id?: string
}

export interface Circle {
  type: 'circle'
  center: Point
  radius: number
  id?: string
}

export interface Rectangle {
  type: 'rectangle'
  origin: Point
  width: number
  height: number
  id?: string
}

export interface Polygon {
  type: 'polygon'
  points: Point[]
  id?: string
}

export interface Arc {
  type: 'arc'
  center: Point
  radius: number
  startAngle: number
  endAngle: number
  id?: string
}

export type Primitive = Line | Circle | Rectangle | Polygon | Arc

export interface Feature {
  id: string
  name: string
  type: 'sketch' | 'extrude' | 'cut' | 'fillet' | 'chamfer' | 'pattern'
  parentId?: string
  children?: string[]
  visible: boolean
  suppressed: boolean
}

export interface ToolDefinition {
  name: string
  description: string
  parameters: ToolParameter[]
}

export interface ToolParameter {
  name: string
  type: 'string' | 'number' | 'boolean' | 'array'
  required: boolean
  description?: string
}
