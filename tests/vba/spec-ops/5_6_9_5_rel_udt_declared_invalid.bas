' INV: vbal.5.6.9.5, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational operator invalid when a declared operand type is a UDT
' ERROR: load
Private Type T
    x As Long
End Type
Public Sub Main()
    Dim u As T, w As T
    Dim r As Boolean
    r = (u = w)
End Sub
