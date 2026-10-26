' INV: vbal.5.4.5.3, stmt.seek
' SOURCE: spec ms-vbal 5.4.5.3 Seek on a number that is not open is error 52
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Seek #9, 1
End Sub
