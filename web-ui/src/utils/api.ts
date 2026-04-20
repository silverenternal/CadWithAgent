import axios from 'axios'
import type { Primitive, Feature } from '../types'

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:8080/api'

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
})

export interface ChatRequest {
  message: string
  conversationId?: string
  primitives?: Primitive[]
}

export interface ChatResponse {
  response: string
  conversationId: string
  actions?: Action[]
}

export interface Action {
  type: 'create_primitive' | 'modify_primitive' | 'delete_primitive' | 'apply_constraint'
  data: any
}

export interface AnalysisResult {
  primitives: Primitive[]
  constraints: Constraint[]
  features: Feature[]
}

export interface Constraint {
  id: string
  type: string
  entities: string[]
  parameters?: Record<string, any>
}

/**
 * Send a chat message to the AI assistant
 */
export async function sendChatMessage(request: ChatRequest): Promise<ChatResponse> {
  const response = await api.post<ChatResponse>('/chat', request)
  return response.data
}

/**
 * Upload a CAD file (SVG, DXF, STEP, IGES)
 */
export async function uploadCadFile(file: File): Promise<AnalysisResult> {
  const formData = new FormData()
  formData.append('file', file)
  
  const response = await api.post<AnalysisResult>('/upload', formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  })
  return response.data
}

/**
 * Export the current model to a specified format
 */
export async function exportModel(format: 'svg' | 'dxf' | 'step' | 'iges'): Promise<Blob> {
  const response = await api.post<Blob>(`/export/${format}`, {}, {
    responseType: 'blob',
  })
  return response.data
}

/**
 * Execute a tool command
 */
export async function executeTool(toolName: string, parameters: Record<string, any>): Promise<any> {
  const response = await api.post('/tools/execute', {
    tool: toolName,
    parameters,
  })
  return response.data
}

/**
 * Get available tools
 */
export async function getAvailableTools(): Promise<any[]> {
  const response = await api.get('/tools')
  return response.data
}

/**
 * Apply constraints to primitives
 */
export async function applyConstraints(constraints: Constraint[]): Promise<Primitive[]> {
  const response = await api.post<{ primitives: Primitive[] }>('/constraints/apply', {
    constraints,
  })
  return response.data.primitives
}

/**
 * Solve the constraint system
 */
export async function solveConstraints(): Promise<{
  primitives: Primitive[]
  status: 'solved' | 'over_constrained' | 'under_constrained' | 'failed'
}> {
  const response = await api.post('/constraints/solve')
  return response.data
}

/**
 * Health check
 */
export async function healthCheck(): Promise<{ status: string; version: string }> {
  const response = await api.get('/health')
  return response.data
}
