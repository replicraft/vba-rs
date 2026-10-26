' INV: vbal.5.2.3.1.3
' SOURCE: spec ms-vbal 5.2.3.1.3 array-dim without bounds-list designates a resizable array
' EXPECT: ub=6
Dim a() As Long
Public Sub Main()
    ReDim a(6)
    Debug.Print "ub=" & CStr(UBound(a))
End Sub
