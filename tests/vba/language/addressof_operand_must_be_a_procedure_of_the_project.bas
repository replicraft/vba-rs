' INV: vbal.5.6.16.8, expr.addressof
' SOURCE: derived 5.6.16.8 the operand of AddressOf names a procedure; a name that is not one is not accepted
' METHOD: by construction
' ERROR: load
Public Sub Main()
    Dim p As LongPtr
    p = AddressOf NoSuchProcedure
End Sub
