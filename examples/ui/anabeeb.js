(function() {

    let binding = undefined;

    const config = {
        ...{
            host: "localhost",
            port: 4040,
        },
        ...(window.anabeeb || {}),
    };

    function toggle(element) {
        const id = element.id;
        const action = element.checked ? 'activate' : 'deactivate';

        post(`http://${config.host}:${config.port}/element/${id}/${action}`);
    }

    function change(element) {
        const id = element.id;
        const value = parseInt(element.value);

        post(`http://${config.host}:${config.port}/element/${id}/change/${value}`);
    }

    function trigger(element) {
        const id = element.id;

        post(`http://${config.host}:${config.port}/element/${id}/trigger`);
    }

    function post(url) {
        fetch(url, {
            method: 'POST',
        })
        .catch((error) => {
            console.error(`Error posting`, error);
            onError(error);
        });
    }

    function updateElement(htmlElement, element) {
        switch (htmlElement.type) {
            case "checkbox":
                htmlElement.checked = (element.active === true);
                break;
            case 'range':
            case 'number':
                htmlElement.min = element.min;
                htmlElement.max = element.max;
                htmlElement.value = element.value;
                break;
        }
    }

    function bind(htmlElement) {
        if (binding) {
            binding.classList.remove("binding");
            binding.disabled = false;
            const id = binding.id;
            post(`http://${config.host}:${config.port}/binding/${id}/end`);
        }

        binding = htmlElement;

        if (binding) {
            binding.classList.add("binding");
            binding.disabled = true;
            const id = binding.id;
            post(`http://${config.host}:${config.port}/binding/${id}/start`);
        }
    }

    function initializeElement(htmlElement) {
        for (const e of [ htmlElement, ...(htmlElement.labels ?? []) ]) {
            e.addEventListener("mousedown", (event) => {
                if (event.ctrlKey || event.metaKey) {
                    event.preventDefault();
                    event.stopImmediatePropagation();

                    bind(htmlElement);
                }
            });
        }

        switch (htmlElement.type) {
            case 'button':
                htmlElement.addEventListener("click", () => trigger(htmlElement));
                return;
            case 'checkbox':
                htmlElement.addEventListener("click", () => toggle(htmlElement));
                break;
            case 'range':
            case 'number':
                htmlElement.addEventListener("input", () => change(htmlElement));
                break;
        }

        const id = htmlElement.id;
        fetch(`http://${config.host}:${config.port}/element/${id}`)
            .then((response) => {
                if (!response.ok) throw new Error(`response is not ok`);
                return response.json();
            })
            .then((data) => {
                updateElement(htmlElement, data.elements[id])
            })
            .catch((error) => {
                console.error(`Error initializing '${id}':`, error);
                onError(error);
            });
    }

    function onError(error) {
        const element = document.getElementById('errors');
        if (element) {
            element.style.display = 'block';
        }
    }

    window.addEventListener('load', () => {
        document.querySelectorAll('[id]').forEach(initializeElement);

        document.addEventListener("mousedown", (event) => {
           bind(undefined);
        });

        const socket = new WebSocket(`ws://${config.host}:${config.port}/ws`, "anabeeb");
        socket.onmessage = function(event) {{
            const data = JSON.parse(event.data);

            if (data.elements) {
                Object.keys(data.elements).forEach(id => {
                    updateElement(document.getElementById(id), data.elements[id]);
                });
            }
        }};
    });

})();