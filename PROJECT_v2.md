The optimization workflow follows these steps:
1. **Image Selection**: User selects images via drag-and-drop or file picker
2. **Task Creation**: Frontend creates optimization tasks with settings and sends them to the backend
3. **Task Processing**: Tasks are validated, batched, and distributed to Sharp workers
4. **Result Handling**: Optimization results return to the frontend with statistics and file paths

```mermaid
%%{init: {'theme': 'base', 'themeVariables': { 'primaryColor': '#f0f0f0', 'primaryTextColor': '#333', 'primaryBorderColor': '#999', 'lineColor': '#666', 'secondaryColor': '#fafafa', 'tertiaryColor': '#f4f4f4'}}}%%
flowchart TD
    %% Define subgraphs for better organization
    subgraph User["🧑‍💻 User Interaction"]
        direction TB
        User1([User]) -->|Selects Images| UI1[Drag & Drop Area]
        User1 -->|Configures Settings| UI2[Settings Panel]
    end
    
    subgraph Frontend["🖥️ Frontend (React)"]
        direction TB
        FE1[File Validation] --> FE2[Task Creation]
        FE2 --> FE3[Tauri Invoke API]
        FE4[Progress Tracking]
        FE5[Results Display]
    end
    
    subgraph Backend["⚙️ Backend (Rust/Tauri)"]
        direction TB
        BE1[Command Handler] --> BE2[Process Pool]
        BE2 --> BE3[Worker Management]
        BE4[Event Handling]
        BE5[Results Aggregation]
    end
    
    subgraph Sidecar["🔄 Sharp Sidecar (Node.js)"]
        direction TB
        SC1[Sharp Sidecar] --> SC2[Worker Threads]
        SC2 --> SC3[Sharp Pipeline]
        SC3 --> SC4[Optimization]
        SC5[Results Collection]
    end
    
    subgraph UI_Updates["📊 UI Updates"]
        direction TB
        UI3[Progress Bar]
        UI4[Results Summary]
    end
    
    %% Connect the subgraphs with labeled edges
    UI1 -->|File Paths| FE1
    UI2 -->|Quality/Format Settings| FE2
    
    FE3 -->|optimize_images Command| BE1
    
    BE3 -->|JSON Task Messages| SC1
    
    SC4 -->|Optimization Results| SC5
    SC5 -->|Progress Events| BE4
    BE4 -->|Tauri Events| FE4
    SC5 -->|Final Results| BE5
    BE5 -->|Command Response| FE5
    
    FE4 -->|Progress Updates| UI3
    FE5 -->|Statistics| UI4
    
    %% File System Operations
    SC4 -.->|Write Optimized Files| FS[(💾 File System)]
    UI4 -.->|Open Result Folder| FS
    
    %% Add a title
    Title[<b>Image Optimizer System Data Flow</b>]
    style Title fill:none,stroke:none,color:#333,font-size:16px
    
    %% Enhanced styling
    classDef userStyle fill:#f5f5f5,stroke:#424242,stroke-width:1px,color:#333,font-weight:bold,rounded:true
    classDef frontendStyle fill:#d4f1f9,stroke:#05a0c8,stroke-width:2px,color:#05506e,font-weight:bold,rounded:true
    classDef backendStyle fill:#ffe6cc,stroke:#f7931e,stroke-width:2px,color:#a85c00,font-weight:bold,rounded:true
    classDef sidecarStyle fill:#e6f5d0,stroke:#8bc34a,stroke-width:2px,color:#3e701a,font-weight:bold,rounded:true
    classDef uiStyle fill:#f9d4e7,stroke:#d81b60,stroke-width:2px,color:#880e4f,font-weight:bold,rounded:true
    classDef storageStyle fill:#e6e6e6,stroke:#757575,stroke-width:2px,color:#424242,font-weight:bold,shape:cylinder
    classDef subgraphStyle fill:#fafafa,stroke:#999,stroke-width:1px,color:#333,font-weight:bold
    
    %% Apply enhanced styles
    class User1 userStyle
    class UI1,UI2,UI3,UI4 uiStyle
    class FE1,FE2,FE3,FE4,FE5 frontendStyle
    class BE1,BE2,BE3,BE4,BE5 backendStyle
    class SC1,SC2,SC3,SC4,SC5 sidecarStyle
    class FS storageStyle
    
    %% Style the subgraphs
    class User,Frontend,Backend,Sidecar,UI_Updates subgraphStyle
```

### 1.2 Frontend (React) 