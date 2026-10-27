' INV: vbal.5.6.16.8, expr.addressof
' SOURCE: derived 5.6.16.8 the value is implementation-defined, but it identifies the procedure: two procedures give different values and one procedure always gives the same
' METHOD: by hand: the comparisons are determined by that identity
' EXPECT: True
' EXPECT: False
Private Function A() As Long
End Function
Private Function B() As Long
End Function
Public Sub Main()
    Dim p As LongPtr, q As LongPtr
    p = AddressOf A
    q = AddressOf B
    Debug.Print p = AddressOf A
    Debug.Print p = q
End Sub
