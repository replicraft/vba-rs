' INV: vbal.5.4.5.9, stmt.write
' SOURCE: spec ms-vbal 5.4.5.9 Null and Error output strings
' EXPECT: [#NULL#,#ERROR 5#]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String, v As Variant
    v = Null
    Open "/t/w.txt" For Output As #1
    Write #1, v, CVErr(5)
    Close #1
    Open "/t/w.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
