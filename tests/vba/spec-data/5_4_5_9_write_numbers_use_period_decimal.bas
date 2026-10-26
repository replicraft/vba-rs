' INV: vbal.5.4.5.9, stmt.write
' SOURCE: spec ms-vbal 5.4.5.9 numbers written with "." decimal separator and no padding
' EXPECT: [-3,0.25,1000000]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/w.txt" For Output As #1
    Write #1, -3, 0.25, 1000000
    Close #1
    Open "/t/w.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
