' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 every numeric non-Date value gets surrounding spaces
' EXPECT: [ 7  8  9  2.5  3.5  12.25 ]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Dim b As Byte, i As Integer, l As Long, g As Single, d As Double, c As Currency
    b = 7: i = 8: l = 9: g = 2.5: d = 3.5: c = 12.25
    Open "/t/p.txt" For Output As #1
    Print #1, b; i; l; g; d; c
    Close #1
    Open "/t/p.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
