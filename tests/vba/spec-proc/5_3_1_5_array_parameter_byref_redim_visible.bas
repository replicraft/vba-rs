' INV: vbal.5.3.1.5, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.5 array parameter is resizable and ByRef by default
' EXPECT: ub=4
Sub Grow(a() As Long)
    ReDim Preserve a(0 To 4)
End Sub
Public Sub Main()
    Dim x() As Long
    ReDim x(0 To 1)
    Grow x
    Debug.Print "ub=" & CStr(UBound(x))
End Sub
