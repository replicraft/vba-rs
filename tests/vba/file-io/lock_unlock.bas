' INV: stmt.lock-unlock, stmt.open
' SOURCE: spec ms-vbal 5.4.5.4 Lock Statement and 5.4.5.5 Unlock Statement
' EXPECT: ok
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/l.bin" For Binary As #1
    Lock #1
    Unlock #1
    Close #1
    Debug.Print "ok"
End Sub
