# ARQHIA

## Descripción

- Un agente de IA que acompaña al usuario desde la definición del proyecto hasta su resolución. Toma en cuenta la experiencia del usuario para realizar el proyecto paso a paso y de forma estable.

- Este producto está hecho para desarrolladores independientes, pequeños grupos e incluso grandes empresas.

- Este agente, a diferencia de otros, no solo escribe y desarrolla código: también ayuda a diseñar un proyecto, sus especificaciones y funcionalidades.

- Acompaña al usuario desde la idea haciendo preguntas pertinentes y creando especificaciones preconfiguradas o generadas por IA. Puede ir desde lo básico (planteamiento) hasta la selección del stack tecnológico.

- Debe ser un agente porque debe desarrollar un proyecto: el usuario debería poder delegar la mayor cantidad de trabajo de forma confiable.

## Flujo

    Creación de proyecto (nombre, descripción, objetivo, funcionalidades) >
    Cuestionario de ARQHIA (genérico + preguntas por nivel + preguntas opcionales
    por IA) > Generación de PROJECT.md + SPECS.md + CONTEXT.md y estructura
    (Project/, ToDo.md, CONTEXT/) > Plan (ROADMAP.md, VERSIONS.md,
    VERSIONS/v0.x.md) > Chat en modos Chat / Plan / Work > Desarrollo y
    modificación del proyecto > versiones estables

## Flujo del Agente

    Petición del usuario > Analista dedicado (revisa CONTEXT.md / PROJECT.md /
    SPECS.md / VERSIONS.md y outlines del código en Project/) > Brief de estado >
    Planner diseña tareas > Orquestador despliega workers > Finaliza tareas >
    Despliega auditor > Guarda errores en TEMP.md > Loop de fixes > Repetir
    hasta que la auditoría quede verde > Commit (rama ARQHIA) > Push (opcional)

## Flujo del agente con STACK de código

    Petición del usuario > Analista (CONTEXT + código) > Petición
    al STACK (nube o local) > Analiza coincidencias > Aprueba y despliega
    workers > Finaliza tareas > Despliega auditor > Guarda errores en TEMP.md >
    Loop de fixes hasta verde > Commit (rama ARQHIA) > Push (opcional) >
    Versión estable > Sube el código con metadatos al STACK

## Capas

### Chat

    Un IDE de agente donde planear o construir el proyecto.

    Archivo / View / API's / Models
    _____________________________________________________________________________________
    Settings                      |
    CONTEXT                       |
    Chat ¬                        |
    1. Concepto                   |
    2. v0.1                       |
    3. Cambio de especificaciones |
                                  |
                                  |
                                  |
                                  | ___________________________________________________________________
                                  | ¿Qué deseas hacer hoy?                                      Build >
                                  |
                                  | ___________________________________________________________________
                                  | Log > Edit File.txt
                                  |       Read File.txt
    Cuenta >

### Cuestionario

    Inicio del proyecto: una caja que va haciendo preguntas genéricas, luego
    preguntas propias del nivel (Principiante / Intermedio / Avanzado) y por
    último preguntas opcionales generadas por IA. Al terminar genera los
    documentos y la estructura del proyecto.

### Estructura del proyecto generado

    {workspace}/
      Project/                 # código + git
      ToDo.md                  # tablero de ejecución
      CONTEXT/
        CONTEXT.md             # índice + estado vivo
        PROJECT.md             # visión, objetivo, alcance, usuario
        SPECS.md               # especificación funcional
        ROADMAP.md             # versiones de alto nivel
        VERSIONS.md            # índice de estado por versión
        VERSIONS/              # un .md por versión

### Stack

    Un sistema (nube + local) que almacena y categoriza archivos de código con
    metadatos: opiniones de usuarios e IA, dependencias, versiones,
    calificación, ejecuciones y bugs reportados.

### Agente

    Un orquestador que analiza el contexto, consulta el STACK, audita su propio
    código, da recomendaciones y consejos.
