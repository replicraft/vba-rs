' INV: vbal.5.4.5.3, stmt.seek
' SOURCE: spec ms-vbal 5.4.5.3 position must have a scalar declared type
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Private Type T
    a As Long
End Type
Public Sub Main()
    Dim u As T
    Open "/s.bin" For Binary As #1
    Seek #1, u
End Sub
