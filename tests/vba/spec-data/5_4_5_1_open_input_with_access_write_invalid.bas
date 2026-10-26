' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Input requires Access Read
' XFAIL: wrong pending
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/o.txt" For Input Access Write As #1
End Sub
