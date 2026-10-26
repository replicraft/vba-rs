' INV: vbal.5.4.5.1.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1.1 file number expression must have a scalar declared type
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Private Type T
    a As Long
End Type
Public Sub Main()
    Dim u As T
    Close #u
End Sub
