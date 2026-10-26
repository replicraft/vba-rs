' INV: expr.addressof, stmt.declare
' SOURCE: unverified real VBA reports a callback with the wrong parameter list as error 49 (bad DLL calling convention) when the stack is unbalanced; vba-rs checks the signature before running the procedure
' EXPECT: 49
' GRANT: native-declare
Private Declare PtrSafe Function EnumWindows Lib "user32" (ByVal lpEnumFunc As LongPtr, ByVal lParam As LongPtr) As Long
Public Function OneParam(ByVal hwnd As LongPtr) As Long
    OneParam = 1
End Function
Public Sub Main()
    On Error Resume Next
    Call EnumWindows(AddressOf OneParam, 0)
    Debug.Print CStr(Err.Number)
End Sub
