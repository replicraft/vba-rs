' INV: vbal.5.4.5.1.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1.1 negative file number is out of range
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim n As Integer
    n = -1
    Open "/t/f" For Output As #n
End Sub
