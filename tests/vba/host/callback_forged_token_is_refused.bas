' INV: expr.addressof, stmt.declare
' SOURCE: unverified a callback pointer is an opaque session token in vba-rs; arithmetic on it, or a guessed number, is error 5 and never runs code
' EXPECT: 5
' EXPECT: 5
' EXPECT: 5
' GRANT: native-declare
Private Declare PtrSafe Function EnumWindows Lib "user32" (ByVal lpEnumFunc As LongPtr, ByVal lParam As LongPtr) As Long
Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Debug.Print "must not run"
    Visit = 1
End Function
Public Sub Main()
    On Error Resume Next
    Call EnumWindows(AddressOf Visit + 4, 0)
    Debug.Print CStr(Err.Number)
    Err.Clear
    Call EnumWindows(AddressOf Visit * 2, 0)
    Debug.Print CStr(Err.Number)
    Err.Clear
    Call EnumWindows(12345, 0)
    Debug.Print CStr(Err.Number)
End Sub
