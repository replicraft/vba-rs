' INV: vbal.5.6.9.8.1, op.not
' SOURCE: spec ms-vbal 5.6.9.8 logical operator invalid when a declared operand type is a UDT
' ERROR: load
Private Type T
    x As Long
End Type
Public Sub Main()
    Dim u As T
    Dim r As Long
    r = Not u
End Sub
