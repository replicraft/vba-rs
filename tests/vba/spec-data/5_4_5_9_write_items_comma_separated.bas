' INV: vbal.5.4.5.9, stmt.write
' SOURCE: spec ms-vbal 5.4.5.9 Write: strings quoted, numbers plain, Booleans #TRUE#/#FALSE#, comma separated
' EXPECT: ["a",1,2.5,#TRUE#,#FALSE#]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/w.txt" For Output As #1
    Write #1, "a", 1, 2.5, True, False
    Close #1
    Open "/t/w.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
