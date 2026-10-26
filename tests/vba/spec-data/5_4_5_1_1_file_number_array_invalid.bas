' INV: vbal.5.4.5.1.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1.1 file number expression must have a scalar declared type
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim a(1) As Long
    Close #a
End Sub
