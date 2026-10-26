' INV: vbal.5.4.5.9, stmt.write
' SOURCE: spec ms-vbal 5.4.5.9 Write takes a marked file number
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/w.txt" For Output As #1
    Write 1, "x"
End Sub
