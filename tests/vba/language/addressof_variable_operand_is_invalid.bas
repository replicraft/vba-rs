' INV: vbal.5.6.16.8, expr.addressof
' SOURCE: derived 5.6.16.8 the operand of AddressOf names a procedure; a variable is not one
' METHOD: by construction
' ERROR: load
Public Sub Main()
    Dim p As LongPtr
    Dim v As Long
    p = AddressOf v
End Sub
