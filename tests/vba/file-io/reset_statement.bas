' INV: stmt.reset
' SOURCE: spec ms-vbal 5.4.5.2 Close and Reset Statements (with 6.1.2.5.1.7 FreeFile)
' EXPECT:  1 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/r.txt" For Output As #1
    Open "/s.txt" For Output As #2
    Reset
    Debug.Print FreeFile
End Sub
