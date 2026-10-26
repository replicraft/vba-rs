' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 As clause with a type is accepted
' EXPECT: 2
Public Sub Main()
    Dim a() As Long
    ReDim a(2) As Long
    Debug.Print CStr(UBound(a))
End Sub
