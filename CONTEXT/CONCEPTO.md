#ARQHIA

##Descripcion:

- Un agente de ia que acompaña al usuario desde la definicion del proyecto hasta su resolucion. Toma en cuenta la experiencia del usuario para realizar el proyecto paso a paso y de forma estable.
 
- Este producto estara hecho para desarrolladores independientes, pequeños grupos e incluso grandes empresas.
 
- Este agente a diferencia de otros no solo se encargara de escribir y desarrollar codigo, tambien podra ayudar a diseñar un proyecto, sus especificaciones y funcionalidades.
 
- Este agente acompaña al usuario en el desarrollo de su proyecto desde la idea, haciendo preguntas pertinentes y creando especificaciones pre-generadas o generadas por ia. Puede ir desde cosas basicas como el planteamiento hasta la seleccion del stack tecnologico.
 
- Este debe ser una agente debido a que debe desarrollar un proyecto, el usuario deberia poder delegar la mayor cantidad de trabajo de manera confiable.
 
##Flujo:
 
    Creacion de proyecto (Nombre, descripcion, preguntas basicas predefinidas) > Cuestionario de ARQHIA (Stack tecnologico, definiciones claras, UI/UX, alcance, etc) > Desarrollo del plan (CONCEPTO.md, ROUTE.md, VERSIONS.md) > Inicio del chat (Dos modos: Plan y Build) > Desarrollo y modificacion del proyecto
    
##Flujo del Agente:

    Peticion del usuario > Revision del CONTEXT (CONCEPTO.md, ROUTE.md, VERSIONS.md) > Revision del codigo > Diseña tareas > Orquestador despliega agentes > Finaliza tareas > Orquestador despliega agentes auditores > Se almacenan los errores en TEMP.md > Orquestador lee y diseña plan > Despliega agentes > Repetir hasta que haya una version estable
    
##Flujo de agente con STACK de Codigo:

    Peticion del usuario > Revision del CONTEXT > Revision del codigo > Peticion al STACK en nube > Analiza coincidencias > Aprueba y despliega agentes > Finaliza tareas > Despliega auditores > Se almacenan errores > Lee y diseña plan > Despliegue de agentes > Loop > Version estable > Sube el codigo con metadatos al STACK
    
##Capas: 

###Chat:
    
    Un IDE de agente donde planear o construir el proyecto.
    
    Archivo / View / API's / Models
    _____________________________________________________________________________________________________________________________________________________________________________________________________________________
    Settings                      |                                                                                                                                                
    CONTEXT                       |
    Chat ¬                        |
    1. Concepto                   |
    2. v0.1                       |
    3. Cambio de especificaciones |
                                  |                                        
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  |
                                  | _________________________________________________________________________________________
                                  | Que deseas hacer hoy?                                                            Build >
                                  |
                                  |
                                  | _____________________________________________________________________________________________________________________________________________________________________________________
                                  | Log > Edit File.txt
                                  |       Read File.txt
                                  |
    Cuenta > 
    
###Cuestionario:

    Inicio del proyecto, una caja en medio que va haciendo preguntas de opcion multiple o de respuesta escrita.
    
###Stack: 

    Un sistema en nube que almacena y categoriza archivos de codigo con metadatos que incluye: Opiniones de usuarios y IA's, dependencias, versiones, calificacion, ejecuciones, bugs reportados.
    
    
###Agente:

    Un orquestador que analiza el contexto, envia peticiones al stack audita su propio codigo, da recomendaciones y consejos.
    
 
    