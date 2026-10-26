' INV: vbal.5.4.5.7, stmt.width
' SOURCE: spec ms-vbal 5.4.5.7 Width takes a marked file number
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/w.txt" For Output As #1
    Width 1, 10
End Sub
