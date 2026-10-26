' INV: vbal.5.4.3.4, stmt.erase
' SOURCE: spec ms-vbal 5.4.3.4 resizable array loses its dimensions
' EXPECT: before=2
' ERROR: 9
Public Sub Main()
    Dim a() As Long
    ReDim a(2)
    Debug.Print "before=" & CStr(UBound(a))
    Erase a
    Debug.Print "after=" & CStr(UBound(a))
End Sub
