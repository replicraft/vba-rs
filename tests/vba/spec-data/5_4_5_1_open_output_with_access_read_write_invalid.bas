' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Output requires access consisting of Write only
' XFAIL: wrong pending
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/o.txt" For Output Access Read Write As #1
End Sub
