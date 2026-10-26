' INV: vbal.5.5.1.2.8, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.8 Let-coercion to and from a UDT, object fields are Set-assigned so both copies refer to the same object
' EXPECT: shared=9
'#MODULE Box class
Public V As Long
'#MODULE Module1 standard
Type Holder
    O As Box
End Type
Public Sub Main()
    Dim a As Holder
    Dim b As Holder
    Set a.O = New Box
    b = a
    b.O.V = 9
    Debug.Print "shared=" & CStr(a.O.V)
End Sub
