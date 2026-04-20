import { useRef } from 'react'
import { useFrame } from '@react-three/fiber'
import * as THREE from 'three'
import type { Primitive } from '../types'
import { useStore } from '../hooks/useStore'

interface CADModelProps {
  primitives: Primitive[]
}

export function CADModel({ primitives }: CADModelProps) {
  const { selectedIds, selectPrimitive, deselectPrimitive } = useStore()
  
  const renderPrimitive = (primitive: Primitive, index: number) => {
    const isSelected = selectedIds.includes(primitive.id || `${index}`)
    const color = isSelected ? '#3b82f6' : '#64748b'
    
    switch (primitive.type) {
      case 'line': {
        const points = [
          new THREE.Vector3(
            primitive.start.x,
            primitive.start.y,
            primitive.start.z || 0
          ),
          new THREE.Vector3(
            primitive.end.x,
            primitive.end.y,
            primitive.end.z || 0
          ),
        ]
        const geometry = new THREE.BufferGeometry().setFromPoints(points)
        return (
          <line
            key={primitive.id || index}
            geometry={geometry}
            onClick={(e) => {
              e.stopPropagation()
              if (isSelected) {
                deselectPrimitive(primitive.id || `${index}`)
              } else {
                selectPrimitive(primitive.id || `${index}`)
              }
            }}
          >
            <lineBasicMaterial attach="material" color={color} linewidth={2} />
          </line>
        )
      }
      
      case 'circle': {
        const points = []
        const segments = 64
        for (let i = 0; i <= segments; i++) {
          const angle = (i / segments) * Math.PI * 2
          points.push(
            new THREE.Vector3(
              primitive.center.x + Math.cos(angle) * primitive.radius,
              primitive.center.y + Math.sin(angle) * primitive.radius,
              primitive.center.z || 0
            )
          )
        }
        const geometry = new THREE.BufferGeometry().setFromPoints(points)
        return (
          <line
            key={primitive.id || index}
            geometry={geometry}
            onClick={(e) => {
              e.stopPropagation()
              if (isSelected) {
                deselectPrimitive(primitive.id || `${index}`)
              } else {
                selectPrimitive(primitive.id || `${index}`)
              }
            }}
          >
            <lineBasicMaterial attach="material" color={color} linewidth={2} />
          </line>
        )
      }
      
      case 'rectangle': {
        const points = [
          new THREE.Vector3(primitive.origin.x, primitive.origin.y, primitive.origin.z || 0),
          new THREE.Vector3(primitive.origin.x + primitive.width, primitive.origin.y, primitive.origin.z || 0),
          new THREE.Vector3(primitive.origin.x + primitive.width, primitive.origin.y + primitive.height, primitive.origin.z || 0),
          new THREE.Vector3(primitive.origin.x, primitive.origin.y + primitive.height, primitive.origin.z || 0),
          new THREE.Vector3(primitive.origin.x, primitive.origin.y, primitive.origin.z || 0),
        ]
        const geometry = new THREE.BufferGeometry().setFromPoints(points)
        return (
          <line
            key={primitive.id || index}
            geometry={geometry}
            onClick={(e) => {
              e.stopPropagation()
              if (isSelected) {
                deselectPrimitive(primitive.id || `${index}`)
              } else {
                selectPrimitive(primitive.id || `${index}`)
              }
            }}
          >
            <lineBasicMaterial attach="material" color={color} linewidth={2} />
          </line>
        )
      }
      
      case 'polygon': {
        if (primitive.points.length < 2) return null
        const points = primitive.points.map(p => 
          new THREE.Vector3(p.x, p.y, p.z || 0)
        )
        // Close the polygon
        points.push(points[0])
        const geometry = new THREE.BufferGeometry().setFromPoints(points)
        return (
          <line
            key={primitive.id || index}
            geometry={geometry}
            onClick={(e) => {
              e.stopPropagation()
              if (isSelected) {
                deselectPrimitive(primitive.id || `${index}`)
              } else {
                selectPrimitive(primitive.id || `${index}`)
              }
            }}
          >
            <lineBasicMaterial attach="material" color={color} linewidth={2} />
          </line>
        )
      }
      
      case 'arc': {
        const points = []
        const segments = 64
        const angleRange = primitive.endAngle - primitive.startAngle
        for (let i = 0; i <= segments; i++) {
          const angle = primitive.startAngle + (i / segments) * angleRange
          points.push(
            new THREE.Vector3(
              primitive.center.x + Math.cos(angle) * primitive.radius,
              primitive.center.y + Math.sin(angle) * primitive.radius,
              primitive.center.z || 0
            )
          )
        }
        const geometry = new THREE.BufferGeometry().setFromPoints(points)
        return (
          <line
            key={primitive.id || index}
            geometry={geometry}
            onClick={(e) => {
              e.stopPropagation()
              if (isSelected) {
                deselectPrimitive(primitive.id || `${index}`)
              } else {
                selectPrimitive(primitive.id || `${index}`)
              }
            }}
          >
            <lineBasicMaterial attach="material" color={color} linewidth={2} />
          </line>
        )
      }
      
      default:
        return null
    }
  }
  
  return (
    <group>
      {primitives.map((primitive, index) => renderPrimitive(primitive, index))}
    </group>
  )
}
