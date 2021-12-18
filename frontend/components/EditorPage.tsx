import monaco from 'monaco-editor'
import Editor from "@monaco-editor/react"
import { useRef, useState, VFC } from "react"
import { Button, Tab, TabList, TabPanel, TabPanels, Tabs, Textarea } from '@chakra-ui/react'
import Head from 'next/head'
import useSWR from 'swr'
import { useRouter } from 'next/router'
import styles from '../styles/Home.module.css'

function generatePdfIframe(s3Url: string): JSX.Element {
  return <iframe src={`${s3Url}/document.pdf`} width="100%" height="100%"></iframe>
}

type EditorPageProps = {
  s3Url?: string,
}

const EditorPage: VFC = (props: EditorPageProps) => {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [pdfPane, setPdfPane] = useState(props.s3Url ? generatePdfIframe(props.s3Url) : <></>)
  const { data: input } = useSWR<string>(props.s3Url ? `${props.s3Url}/input.saty` : null)
  const { data: stdout } = useSWR<string>(props.s3Url ? `${props.s3Url}/stdout.txt` : null)
  const { data: stderr } = useSWR<string>(props.s3Url ? `${props.s3Url}/stderr.txt` : null)
  const router = useRouter()

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
        const { status, s3_url: s3Url }: { status: number, s3_url: string } = await response.json();

        if (status === 0) {
          setPdfPane(generatePdfIframe(s3Url));
        }

        const buildId = s3Url.split("/").pop()
        if (buildId !== undefined) {
          router.push(`/share/${buildId}`)
        }
      } finally {
        setIsLoading(false);
      }
    }
  }

  return (
    <div className={styles.container}>
      <Head>
        <title>SATySFi Playground</title>
        <meta name="description" content="Playground for SATySFi programming/typesetting language" />
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
          value={input ?? "Enter SATySFi program here"}
          theme="vs-dark"
          options={{
            fontSize: 16,
          }}
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

export default EditorPage