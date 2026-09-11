# Proyecto ARQHIA

ARQHIA es un agente de IA nativo que acompaña al usuario desde la definición de la idea hasta la resolución estable del proyecto. A diferencia de otros asistentes que solo generan código, ARQHIA ayuda a diseñar el proyecto, sus especificaciones, funcionalidades y stack, adaptándose al nivel de experiencia del usuario y trabajando de forma delegable y estable paso a paso.

## Características principales

- **Orquestación completa**: Desde la idea hasta la implementación real del proyecto
- **Multi-provider LLM**: Soporte para OpenAI, Anthropic, OpenRouter y LM Studio
- **Agente autónomo**: Capaz de ejecutar tareas en un workspace estructurado
- **Seguridad y permisos**: Control detallado de accesos y herramientas disponibles
- **Workspace gestionado**: Cada proyecto tiene un espacio de trabajo dedicado con archivos y herramientas
- **Flujo guiado**: Cuestionario para definir especificaciones y planes de acción

## Stack tecnológico

- **Frontend**: Iced 0.13 (Rust puro, sin WebView)
- **Backend**: Rust + Tokio + SQLite para persistencia
- **LLM**: Clientes nativos para múltiples proveedores con streaming
- **Herramientas**: CRUD de archivos, ejecución de comandos con allowlist, búsqueda de archivos

## Funcionalidades

1. **Creación de proyectos**: Con espacio de trabajo dedicado
2. **Chat IA**: Conversaciones con múltiples proveedores LLM
3. **Agente de código**: Ejecución de tareas en el workspace con permisos controlados
4. **Cuestionario guiado**: Definición de especificaciones del proyecto
5. **Gestión de permisos**: Control de acceso a herramientas peligrosas
6. **STACK local**: Base de conocimiento para reutilización de código# ARQHIA
