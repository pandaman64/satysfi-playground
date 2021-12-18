import Editor from '@monaco-editor/react'
import type { NextPage } from 'next'
import Head from 'next/head'
import { useRef, useState } from 'react'
import styles from '../styles/Home.module.css'
import monaco from 'monaco-editor'
import { Button, Tab, TabList, TabPanel, TabPanels, Tabs, Textarea } from '@chakra-ui/react'

const Home: NextPage = () => {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [pdfPane, setPdfPane] = useState(<div></div>)
  const [stdout, setStdout] = useState("")
  const [stderr, setStderr] = useState("")

  async function onRun() {
    if (editorRef.current !== null) {
      setIsLoading(true);
      try {
        const source = editorRef.current.getValue();
        const body = {
          source: source,
        };
        const response = await fetch("http://localhost:8080/persist", {
          method: "POST",
          mode: "cors",
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify(body),
        });
        const { status, s3_url } = await response.json();

        if (status === 0) {
          setPdfPane(<iframe src={`${s3_url}/document.pdf`} width="100%" height="100%"></iframe>);
        }
        const [stdout, stderr] = await Promise.allSettled([
          fetch(`${s3_url}/stdout.txt`).then(response => response.text()),
          fetch(`${s3_url}/stderr.txt`).then(response => response.text()),
        ]);
        if (stdout.status === "fulfilled") {
          setStdout(stdout.value);
        }
        if (stderr.status === "fulfilled") {
          setStderr(stderr.value);
        }
      } finally {
        setIsLoading(false);
      }
    }
  }

  return (
    <div className={styles.container}>
      <Head>
        <title>Create Next App</title>
        <meta name="description" content="Generated by create next app" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <div className={styles.header}>
        <Button isLoading={isLoading} size="lg" colorScheme="blue" onClick={onRun}>
          Run
        </Button>
      </div>

      <div className={styles.editor}>
        <Editor
          width="50%"
          height="100%"
          defaultLanguage=""
          defaultValue="// some comment"
          theme="vs-dark"
          options={{
            fontSize: 16,
          }}
          // declaring second argument makes Next unhappy. why?
          onMount={(editor) => { editorRef.current = editor }}
        />
        <Tabs variant="line" isFitted width="50%" height="100%" display="flex" flexDirection="column">
          <TabList>
            <Tab>PDF</Tab>
            <Tab>stdout</Tab>
            <Tab>stderr</Tab>
          </TabList>
          <TabPanels flex={1}>
            <TabPanel padding={0} height="100%">{pdfPane}</TabPanel>
            <TabPanel padding={0} height="100%">
              <Textarea isReadOnly resize="none" width="100%" height="100%" value={stdout}></Textarea>
            </TabPanel>
            <TabPanel padding={0} height="100%">
              <Textarea isReadOnly resize="none" width="100%" height="100%" value={stderr}></Textarea>
            </TabPanel>
          </TabPanels>
        </Tabs>
      </div>
    </div>
  )
}

export default Home