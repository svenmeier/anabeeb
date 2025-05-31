(function() {

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

        fetch(`http://${config.host}:${config.port}/${id}/${action}`, {
            method: 'POST',
        });
    }

    function change(element) {
        const id = element.id;
        const value = parseInt(element.value);

        fetch(`http://${config.host}:${config.port}/${id}/change/${value}`, {
            method: 'POST',
        });
    }

    function trigger(element) {
        const id = element.id;

        fetch(`http://${config.host}:${config.port}/${id}/trigger`, {
            method: 'POST',
        });
    }

    function updateElement(htmlElement, element) {
        if (htmlElement.type === 'checkbox') {
            htmlElement.checked = (element.active === true);
        } else if (htmlElement.type === 'range') {
            htmlElement.min = element.min;
            htmlElement.max = element.max;
            htmlElement.value = element.value;
        }
    }

    function initializeElement(element) {
        if (element.type === 'button') {
            element.addEventListener("click", () => trigger(element));
            return;
        } else if (element.type === 'checkbox') {
            element.addEventListener("click", () => toggle(element));
        } else if (element.type === 'range') {
            element.addEventListener("input", () => change(element));
        }

        fetch(`http://${config.host}:${config.port}/${element.id}`)
            .then((response) => {
                if (!response.ok) throw new Error(`not ok`);
                return response.json();
            })
            .then((data) => {
                updateElement(element, data[element.id])
            })
            .catch((error) => console.error(`Error initializing ${id}:`, error));
    }

    window.addEventListener('load', () => {
        document.querySelectorAll("input").forEach(initializeElement);

        const socket = new WebSocket(`ws://${config.host}:${config.port}/ws`, "anabeeb");
        socket.onmessage = function(event) {{
            const data = JSON.parse(event.data);

            Object.keys(data).forEach(id => {
                updateElement(document.getElementById(id), data[id]);
            });
        }};
    });

})();