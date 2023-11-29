# Casos de Prueba

<p align="left">
    <img alt="arquitectura" src="ferris2.gif" height="5%"/>
</p>

---

## Modelos de Stocks

### Stock 1 [stock1.txt]
- Producto 1: 5 unidades
- Producto 2: 5 unidades
- Producto 3: 5 unidades

### Stock 2 [stock2.txt]
- Producto 2: 10 unidades
- Producto 4: 10 unidades
- Producto 5: 10 unidades
- Producto 6: 10 unidades

### Stock 3 [stock3.txt]
- Producto 1: 10 unidades
- Producto 3: 10 unidades
- Producto 5: 10 unidades
- Producto 6: 10 unidades

## Modelo de Órdenes

### Archivo de órdenes 1 [orders1.txt]
- Orden 1
  - Producto 1: 3 unidades
  - Producto 2: 5 unidades
- Orden 2
  - Producto 3: 7 unidades
- Orden 3
  - Producto 1: 2 unidades
  - Producto 3: 5 unidades

### Archivo de órdenes 2 [orders2.txt]
- Orden 1
  - Producto 2: 2 unidades
  - Producto 4: 2 unidades
- Orden 2
  - Producto 6: 2 unidades
- Orden 3
  - Producto 2: 2 unidades
  - Producto 6: 2 unidades
  
### Archivo de órdenes 3 [orders3.txt]
- Orden 1
  - Producto 1: 2 unidades
  - Producto 3: 2 unidades
- Orden 2
  - Producto 5: 2 unidades
- Orden 3
  - Producto 1: 2 unidades
  - Producto 6: 2 unidades

### Archivo de órdenes 4 [orders4.txt]
- Orden 1 
  - Producto 1: 2 unidades
  - Producto 2: 2 unidades
- Orden 2
  - Producto 3: 2 unidades
  - Producto 6: 2 unidades

### Archivo de órdenes 5 [orders5.txt]
- Orden 1
  - Producto 4: 2 unidades
  - Producto 5: 2 unidades
- Orden 2
  - Producto 6: 2 unidades

## Casos de prueba

### Caso 1: “Procesamiento de órdenes de un local”

#### Actores:
- Base de Datos
- E-commerce: 
  - Órdenes: -
  - Workers: DEFAULT (3)
- Local: 
  - Órdenes: [Archivo de órdenes 1](#archivo-de-órdenes-1-orders1txt)
  - Stock: [Stock 1](#stock-1-stock1txt)
  - Workers: DEFAULT (3)

#### Resultado esperado:
- [Órden 1](#archivo-de-órdenes-1-orders1txt): Procesada, completada satisfactoriamente e informada hacia el e-commerce y Base de Datos como órden local.
- [Órden 2](#archivo-de-órdenes-1-orders1txt): Procesada, cancelada y no informada a la Base de Datos.
- [Órden 3](#archivo-de-órdenes-1-orders1txt): Procesada, completada satisfactoriamente e informada hacia el e-commerce y Base de Datos como órden local.

### Caso 2: “Manejo general con elección de líder (correr varios e-commerces con varios locales y ver que se desconecta y reconecta todo como es debido)”

#### Actores:
- Base de Datos
- E-commerce (múltiples): 
  - Órdenes: -
  - Workers: DEFAULT (3)
- Local (múltiples): 
  - Órdenes: -
  - Stock: -
  - Workers: DEFAULT (3)

#### Casos y resultados:
- Arrancar un nuevo e-commerce líder y todos los locales se conectan a él.
- Cortar el e-commerce no líder y reconectarlo (no debería afectar a los demás).
- Cortar e-commerce líder, locales se conectan al siguiente id más bajo, reconectar líder y ver que se conectan de vuelta.

### Caso 3: “Procesamiento de órdenes de e-commerce principal”

#### Actores:
- Base de Datos
- E-commerce: 
  - Órdenes: [Archivo de órdenes 1](#archivo-de-órdenes-1-orders1txt)
  - Workers: DEFAULT (3)
- Local: 
  - Órdenes: -
  - Stock: [Stock 1](#stock-1-stock1txt)
  - Workers: DEFAULT (3)

#### Resultado esperado:
- [Órden 1.1](#archivo-de-órdenes-1-orders1txt): Procesada, reservada, posiblemente completada (según valor random) e informada hacia el e-commerce y Base de Datos como órden web.
- [Órden 1.2](#archivo-de-órdenes-1-orders1txt): Procesada, reservada, posiblemente completada (según valor random) e informada hacia el e-commerce y Base de Datos como órden web.
- [Órden 2](#archivo-de-órdenes-1-orders1txt): Procesada, cancelada e informada hacia el e-commerce y Base de Datos como órden web.
- [Órden 3.1](#archivo-de-órdenes-1-orders1txt): Procesada, reservada, posiblemente completada (según valor random) e informada hacia el e-commerce y Base de Datos como órden web.
- [Órden 3.2](#archivo-de-órdenes-1-orders1txt): Procesada, reservada, posiblemente completada (según valor random) e informada hacia el e-commerce y Base de Datos como órden web.

### Caso 4: “Procesamiento de órdenes de e-commerce secundario”

#### Actores:
- Base de Datos
- E-commerce principal:
  - Órdenes: -
  - Workers: DEFAULT (3)
- E-commerce secundario: 
  - Órdenes: [Archivo de órdenes 1](#archivo-de-órdenes-1-orders1txt)
  - Workers: DEFAULT (3)
- Local: 
  - Órdenes: -
  - Stock: [Stock 1](#stock-1-stock1txt)
  - Workers: DEFAULT (3)

#### Resultado esperado:
- Considerar que todas las consultas y órdenes deben pasar por el e-commerce principal.
- [Órden 1.1](#archivo-de-órdenes-1-orders1txt): Procesada, reservada, posiblemente completada (según valor random) e informada hacia el e-commerce y Base de Datos como órden web.
- [Órden 1.2](#archivo-de-órdenes-1-orders1txt): Procesada, reservada, posiblemente completada (según valor random) e informada hacia el e-commerce y Base de Datos como órden web.
- [Órden 2](#archivo-de-órdenes-1-orders1txt): Procesada, cancelada e informada hacia el e-commerce y Base de Datos como órden web.
- [Órden 3.1](#archivo-de-órdenes-1-orders1txt): Procesada, reservada, posiblemente completada (según valor random) e informada hacia el e-commerce y Base de Datos como órden web.
- [Órden 3.2](#archivo-de-órdenes-1-orders1txt): Procesada, reservada, posiblemente completada (según valor random) e informada hacia el e-commerce y Base de Datos como órden web.

### Caso 5: “Procesamiento de ordenes de e-commerce y local”

#### Actores:
- Base de Datos
- E-commerce principal:
  - Órdenes: [Archivo de órdenes 4](#archivo-de-órdenes-4-orders4txt)
  - Workers: 1
- E-commerce secundario: 
  - Órdenes: [Archivo de órdenes 5](#archivo-de-órdenes-5-orders5txt)
  - Workers: 2
- Local 1: 
  - Órdenes: [Archivo de órdenes 2](#archivo-de-órdenes-2-orders2txt)
  - Stock: [Stock 2](#stock-2-stock2txt)
  - Workers: 2
- Local 2:
  - Órdenes: [Archivo de órdenes 3](#archivo-de-órdenes-3-orders3txt)
  - Stock: [Stock 3](#stock-3-stock3txt)
  - Workers: 1

#### Resultado esperado:
Se espera que todas las órdenes sean finalizadas exitosamente.

### Caso 6: “Procesamiento de órdenes de ambos y corte de conexión en local que recibió y procesó órdenes”

#### Actores:
- Base de Datos
- E-commerce principal:
  - Órdenes: [Archivo de órdenes 4](#archivo-de-órdenes-4-orders4txt)
  - Workers: 1
- E-commerce secundario:
  - Órdenes: [Archivo de órdenes 5](#archivo-de-órdenes-5-orders5txt)
  - Workers: 2
- Local 1: 
  - Órdenes: [Archivo de órdenes 2](#archivo-de-órdenes-2-orders2txt)
  - Stock: [Stock 2](#stock-2-stock2txt)
  - Workers: 2
- Local 2:
  - Órdenes: [Archivo de órdenes 3](#archivo-de-órdenes-3-orders3txt)
  - Stock: [Stock 3](#stock-3-stock3txt)
  - Workers: 1

### Resultado esperado:
Se espera que el local continúe procesando órdenes locales y órdenes ya recibidas web luego de su desconexión. En el momento en que el local es conectado nuevamente a la red, el mismo debería informar todas las órdenes que fueron realizadas en ese intervalo de tiempo y se debe informar a la base de datos sobre los completados para que pueda mantener el stock de los locales actualizado.

### Caso 7: “Procesamiento de órdenes de ambos y corte de conexión en el e-commerce secundario”

#### Actores:
- Base de Datos
- E-commerce principal:
  - Órdenes: [Archivo de órdenes 4](#archivo-de-órdenes-4-orders4txt)
  - Workers: 1
- E-commerce secundario:
  - Órdenes: [Archivo de órdenes 5](#archivo-de-órdenes-5-orders5txt)
  - Workers: 2
- Local 1: 
  - Órdenes: [Archivo de órdenes 2](#archivo-de-órdenes-2-orders2txt)
  - Stock: [Stock 2](#stock-2-stock2txt)
  - Workers: 2
- Local 2:
  - Órdenes: [Archivo de órdenes 3](#archivo-de-órdenes-3-orders3txt)
  - Stock: [Stock 3](#stock-3-stock3txt)
  - Workers: 1

#### Resultado esperado:
El objetivo principal es que se haga una elección de leader en el medio de procesamiento de órdenes de múltiples locales e e-commerces a partir de una desconexión del e-commerce principal. Se espera que los resultados de las órdenes emitidas previo a la desconexión queden almacenadas en el nuevo e-commerce principal. Luego de que se vuelva a levantar la conexión, se debe asignar como leader al e-commerce que estaba sin conexión y además devolverle el resultado de las órdenes que no pudo recibir mientras se encontraba desconectado.

### Caso 8: “Procesamiento de órdenes de ambos y corte de conexión en el e-commerce secundario justo antes de recibir la orden del líder”
#### Actores:
- Base de Datos
- E-commerce principal:
  - Órdenes: -
  - Workers: DEFAULT (3)
- E-commerce secundario: 
  - Órdenes: [Archivo de órdenes 5](#archivo-de-órdenes-5-orders5txt)
  - Workers: 2
- Local 1: 
  - Órdenes: [Archivo de órdenes 2](#archivo-de-órdenes-2-orders2txt)
  - Stock: [Stock 2](#stock-2-stock2txt)
  - Workers: 2
- Local 2:
  - Órdenes: [Archivo de órdenes 3](#archivo-de-órdenes-3-orders3txt)
  - Stock: [Stock 3](#stock-3-stock3txt)
  - Workers: 1
    
#### Resultado esperado:
El objetivo principal es que el e-commerce principal haga llegar órdenes de un e-commerce secundario a alguno de los locales y se le interrumpa la conexión al e-commerce secundario. Esto generará que luego de haberse completados las órdenes en los locales, las mismas no encuentren el e-commerce que las generó y por lo tanto tengan que ser almacenadas en el actual e-commerce principal. Al momento de reconectarse el e-commerce secundario, se espera que el e-commerce principal le dé todas las órdenes, preguntas de stock y órdenes no tomadas por locales, dado que cada uno de los e-commerce tiene un back up de las transacciones no entregadas empaquetadas por e-commerce.
