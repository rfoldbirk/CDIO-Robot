import express from "express";
import { ChildProcess, spawn } from "child_process";
const app = express();
const port = 3000;

let loops: NodeJS.Timeout[] = [];

app.get("/", (req, res) => {
    res.sendFile(process.cwd() + "/public/index.html");
});

app.get("/style.css", (req, res) => {
    res.sendFile(process.cwd() + "/public/style.css");
});


app.get('/balls', (req, res) => {
    const { filename } = req.query;
    console.log(" -> Filename:", filename);
    let process = spawn('../track/target/release/track', [
        '../analyzer/images/' + filename,
    ]);
    react_to_process('BALLS', process, res);
});

function start() {
    loops.push(setInterval(async () => {
        let process = spawn('../yolo')
    }, 500));
}

function stop() {
    for (let loop of loops) {
        clearTimeout(loop);
    }
}


app.get('/obstacles', (req, res) => {
    const { filename, white, orange, red, wp, op, rp } = req.query;
    let a = spawn("../analyzer/target/release/track-analyzer", [
        '../analyzer/images/' + filename,
        `${white}`,
        `${orange}`,
        `${red}`,
        `${wp}`,
        `${op}`,
        `${rp}`
    ]);

    react_to_process('OBSTACLES', a, res);
})


app.get('path', (req, res) => {
    let process = spawn('../path/target/release/path');

    process.stdout.on('data', data => {
        console.log('[PATH]:', String(data));
    })
})



app.get("/latest.png", (req, res) => {
    res.sendFile(process.cwd() + "/out.png");
});

app.listen(port, () => {
    console.log(`Example app listening on port ${port}`);
});



function react_to_process(name: string, process: ChildProcess, res: any) {
    let failsafe = setTimeout(() => res.send({ok: false}), 600);
    process.stdout?.on('data', data => {
        console.log('['+name+']:', String(data));
        if (data.includes("executed in: ")) {
            const time = String(data).split('executed in: ')[1].trim();
            clearTimeout(failsafe)
            try {
                res.send({ ok: true, time })
            } catch (e) {
                console.log("fuck dig")
            }
        }
    })

    process.stderr?.on("data", data => {
        console.log(`[${name}]: data: ${data}`);
    })
}
