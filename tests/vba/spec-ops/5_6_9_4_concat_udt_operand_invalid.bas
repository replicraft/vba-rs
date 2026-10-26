' INV: vbal.5.6.9.4, op.concat
' SOURCE: spec ms-vbal 5.6.9.4 & operator, UDT declared operand is invalid
' ERROR: load
Private Type T
    x As Long
End Type
Public Sub Main()
    Dim u As T
    Dim s As String
    s = "x" & u
End Sub
