' INV: vbal.5.4.3.4, stmt.erase
' SOURCE: spec ms-vbal 5.4.3.4 erased resizable array returns to initial state
' EXPECT: 4 0
Public Sub Main()
    Dim a() As Long
    ReDim a(2)
    a(1) = 3
    Erase a
    ReDim a(4)
    Debug.Print CStr(UBound(a)) & " " & CStr(a(1))
End Sub
