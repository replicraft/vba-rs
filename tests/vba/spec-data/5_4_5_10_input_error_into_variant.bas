' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 #ERROR n# yields an error value
' EXPECT: True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim v As Variant
    Open "/t/i.txt" For Output As #1
    Print #1, "#ERROR 7#"
    Close #1
    Open "/t/i.txt" For Input As #1
    Input #1, v
    Close #1
    Debug.Print CStr(IsError(v))
End Sub
