import express from "express";
import { spawn } from "child_process";
const app = express();
const port = 3000;

app.get("/", (req, res) => {
    res.sendFile(process.cwd() + "/public/index.html");
});

app.get("/style.css", (req, res) => {
    res.sendFile(process.cwd() + "/public/style.css");
});

app.get("/go", (req, res) => {
    const { wr, wg, wb, or, og, ob, wp, op } = req.query;
    let a = spawn("../analyzer/target/release/track-analyzer", ["../analyzer/images/14.jpg", `${wr}`, `${wg}`, `${wb}`, `${or}`, `${og}`, `${ob}`, `${wp}`, `${op}`]);

    let failsafe = setTimeout(() => {
        res.send({ok: false})
    }, 300)

    a.stdout.on("data", (data) => {
        if (data.includes("executed in: ")) {
            const time = String(data).split('executed in: ')[1].trim();

            clearTimeout(failsafe)
            res.send({ ok: true, time })
        }
    });

    a.stderr.on("data", (data) => {
        console.log(`data: ${data}`)
    })
});

app.get("/latest.png", (req, res) => {
    res.sendFile(process.cwd() + "/out.png");
});

app.listen(port, () => {
    console.log(`Example app listening on port ${port}`);
});
