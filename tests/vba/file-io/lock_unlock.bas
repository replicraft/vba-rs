' INV: stmt.lock-unlock, stmt.open
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/lock-unlock-statements
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
