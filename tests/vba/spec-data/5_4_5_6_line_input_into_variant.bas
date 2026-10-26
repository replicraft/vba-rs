' INV: vbal.5.4.5.6, stmt.line-input
' SOURCE: spec ms-vbal 5.4.5.6 variable may be Variant
' EXPECT: [text] 8
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim v As Variant
    Open "/t/li.txt" For Output As #1
    Print #1, "text"
    Close #1
    Open "/t/li.txt" For Input As #1
    Line Input #1, v
    Close #1
    Debug.Print "[" & v & "] " & CStr(VarType(v))
End Sub
