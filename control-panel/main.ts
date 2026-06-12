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
    const { filename, white, orange, red, wp, op, rp } = req.query;

    console.log(req.query);
    console.log(filename, white, orange, red, wp, op, rp );
    
    let a = spawn("../analyzer/target/release/track-analyzer", [
        "../analyzer/images/" + filename,
        `${white}`,
        `${orange}`,
        `${red}`,
        `${wp}`,
        `${op}`,
        `${rp}`
    ]);

    let failsafe = setTimeout(() => {
        res.send({ok: false})
    }, 300)

    a.stdout.on("data", (data) => {
        console.log(String(data));
        if (data.includes("executed in: ")) {
            const time = String(data).split('executed in: ')[1].trim();

            clearTimeout(failsafe)

            try {
                res.send({ ok: true, time })
            } catch (e) {
                console.log("fuck dig")
            }
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
