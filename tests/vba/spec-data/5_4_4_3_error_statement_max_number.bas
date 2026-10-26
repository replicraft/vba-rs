' INV: vbal.5.4.4.3, stmt.error
' SOURCE: spec ms-vbal 5.4.4.3 numbers up to 65535 are valid
' ERROR: 65535
Public Sub Main()
    Error 65535
End Sub
