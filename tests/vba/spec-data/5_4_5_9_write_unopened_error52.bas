' INV: vbal.5.4.5.9, stmt.write
' SOURCE: spec ms-vbal 5.4.5.9 Write to a number that is not open is error 52
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Write #6, "x"
End Sub
