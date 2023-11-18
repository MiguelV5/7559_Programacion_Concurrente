[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-24ddc0f5d75046c5622901739e7c5dd533143b0c8e959d652212380cedb1ea36.svg)](https://classroom.github.com/a/AdDZ0HGe)

---

# [75.59] TP Ferris-Commerce - Tecnicas de Programacion Concurrente I - 1c2023

---

<br>
<p align="center">
  <img src="https://raw.githubusercontent.com/MiguelV5/MiguelV5/main/misc/logofiubatransparent_partialwhite.png" width="50%"/>
</p>
<br>

---

## Grupo Ferris-Appreciators-2

### Integrantes

| Nombre                                                              | Padrón |
| ------------------------------------------------------------------- | ------ |
| [Luciano Gamberale](https://github.com/lucianogamberale)            | 105892 |
| [Erick Martinez](https://github.com/erick12m)                       | 103745 |
| [Miguel Vasquez](https://github.com/MiguelV5)                       | 107378 |

---

## Introducción

El presente trabajo práctico tiene como objetivo la creación de software para el manejo de stocks de una cadena de tiendas distribuida por todo el país.

Las aplicaciones simulan el manejo desde el sitio de e-commerce, además de locales físicos que pueden operar independientemende del estado de conexión actual del local. 

Puede encontrar el enunciado [aquí](https://concurrentes-fiuba.github.io/2C2023_tp.html). 

## Ejecución:

Tanto el e-commerce como los locales físicos se pueden ejecutar de la siguiente manera:

```bash
cargo run -p <e_commerce|local_shop> -- <orders file>
```
Ambos a su vez proveen comandos para interactuar con el sistema durante la ejecución:
- e_commerce:
    - `exit`: cierra el e-commerce de forma segura.
    - `push`: comienza el procesado de las ordenes recibidas.
    - `list_locals`: lista los locales y su estado actual (conectado/desconectado).
- local_shop:
    - `exit`: cierra el local de forma segura.
    - `push`: comienza el procesado de las ordenes recibidas.
    - `cc`: cierra la conexión con el e-commerce. El proceso sigue activo.
    - `rc`: reestablece la conexión con el e-commerce (las conexiones se intentan reestablecer automaticamente cada 20 segundos).

## Tests

Para correr los tests, ejecutar:

```bash
cargo test
```
---

## Informe

### Arquitectura

#### General

...

<!-- imagen de la arquitectura -->
<p align="center">
    <img alt="arquitectura" src="./assets/imgs/arquitectura.png" width="100%"/>
</p>

#### E-commerce

Los nodos de e-commerce se encargan de recibir ordenes de compra y distribuirlas a los locales físicos. Para esto, se utiliza un sistema de colas de mensajes, donde cada local tiene su propia cola. Este sistema es manejado por medio de la abstracción de Actores que provee la librería [Actix](https://actix.rs/). 

En particular se implementan los siguientes actores:
- `SLMiddleman`: se encarga de recibir las ordenes de compra y distribuirlas a los locales físicos, manejar el estado de conexión con los mismos, y reenviar resultados recibidos desde los locales al actor `OrderHandler`.
- `SSMiddleman`: se encarga de manejar el estado de conexión con los demas nodos de e-commerce, incluyendo manejo de algoritmo de eleccion de lider y reenvio de resultados de procesamiento de ordenes al nodo que la solicito.
- `OrderHandler`: se encarga de reenviar mensajes relacionados a las ordenes solicitadas y a resultados de las mismas al actor correspondiente, segun sea para delegar la orden a un local (pasando por el `SLMiddleman`) o para reenviar un resultado de procesamiento hacia el `SSMiddleman`.

...

<p align="center">
    <img alt="e_commerce" src="./assets/imgs/e_commerce.png" width="100%"/>
</p>

#### Local shop

Los nodos de local shop se encargan de recibir ordenes de compra y procesarlas, de dos fuentes distintas: peticiones de e-commerce y ordenes locales. 

Se implementan los siguientes actores:
- `StockHandler`: se encarga de manejar el stock de productos del local, y de responder a consultas de disponibilidad de stock, asi como peticiones de reserva.
- `OrderWorker`: se encarga de procesar una orden de compra, ya sea local o de e-commerce. Para esto, se comunica con el actor `StockHandler` para verificar disponibilidad de stock, y con el actor `OrderHandler` para notificar resultados.` 
- `OrderHandler`: se encarga de reenviar mensajes relacionados a todas las ordenes solicitadas al actor correspondiente, segun sea para delegar el procesamiento de una orden o para reenviar un resultado de procesamiento hacia el `SLMiddleman`.
- `SLMiddleman`: se encarga de manejar el estado de conexión con el e-commerce, de reenviarle al `OrderHandler` las peticiones obtenidas de dicha conexión, y de reenviar los resultados de procesamiento de ordenes al mismo.

...

<p align="center">
    <img alt="local_shop" src="./assets/imgs/local_shop.png" width="100%"/>
</p>

...
