' INV: vbal.5.4.5.7, stmt.width
' SOURCE: spec ms-vbal 5.4.5.7 line width above 255 is error 5
' XFAIL: wrong pending
' ERROR: 5
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/w.txt" For Output As #1
    Width #1, 256
End Sub
