import { useState, useEffect } from 'react'
import { Canvas } from '@react-three/fiber'
import { OrbitControls, GridHelper, AxesHelper } from '@react-three/drei'
import { Toolbar } from './components/Toolbar'
import { ChatPanel } from './components/ChatPanel'
import { FeatureTree } from './components/FeatureTree'
import { PropertiesPanel } from './components/PropertiesPanel'
import { useStore } from './hooks/useStore'
import { CADModel } from './components/CADModel'

function App() {
  const { primitives, darkMode } = useStore()
  const [showChat, setShowChat] = useState(true)
  const [showFeatureTree, setShowFeatureTree] = useState(true)
  const [showProperties, setShowProperties] = useState(true)

  return (
    <div className={`h-screen w-screen flex flex-col ${darkMode ? 'dark' : ''}`}>
      {/* Toolbar */}
      <Toolbar 
        onToggleChat={() => setShowChat(!showChat)}
        onToggleFeatureTree={() => setShowFeatureTree(!showFeatureTree)}
        onToggleProperties={() => setShowProperties(!showProperties)}
      />

      {/* Main content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left sidebar - Feature Tree */}
        {showFeatureTree && (
          <aside className="w-64 border-r bg-card overflow-hidden">
            <FeatureTree />
          </aside>
        )}

        {/* Center - 3D Viewer */}
        <main className="flex-1 relative bg-background">
          <Canvas
            camera={{ position: [5, 5, 5], fov: 50 }}
            shadows
            dpr={[1, 2]}
          >
            <color attach="background" args={darkMode ? ['#1a1a2e'] : ['#f5f5f5']} />
            <ambientLight intensity={0.5} />
            <directionalLight
              position={[10, 10, 5]}
              intensity={1}
              castShadow
              shadow-mapSize-width={2048}
              shadow-mapSize-height={2048}
            />
            
            {/* Grid and Axes */}
            <GridHelper args={[20, 20, '#444', '#666']} />
            <AxesHelper args={[2]} />

            {/* CAD Model */}
            <CADModel primitives={primitives} />

            {/* Camera Controls */}
            <OrbitControls
              enablePan={true}
              enableZoom={true}
              enableRotate={true}
              minDistance={1}
              maxDistance={100}
            />
          </Canvas>

          {/* Quick actions overlay */}
          <div className="absolute bottom-4 left-4 flex gap-2">
            <button className="px-3 py-2 bg-primary text-primary-foreground rounded-md text-sm hover:bg-primary/90">
              Fit View
            </button>
            <button className="px-3 py-2 bg-secondary text-secondary-foreground rounded-md text-sm hover:bg-secondary/90">
              Top View
            </button>
            <button className="px-3 py-2 bg-secondary text-secondary-foreground rounded-md text-sm hover:bg-secondary/90">
              Front View
            </button>
            <button className="px-3 py-2 bg-secondary text-secondary-foreground rounded-md text-sm hover:bg-secondary/90">
              Right View
            </button>
          </div>
        </main>

        {/* Right sidebar - Properties & Chat */}
        <div className="flex flex-col border-l">
          {showProperties && (
            <div className="w-80 border-b">
              <PropertiesPanel />
            </div>
          )}
          
          {showChat && (
            <div className="w-80 flex-1 overflow-hidden">
              <ChatPanel />
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

export default App
