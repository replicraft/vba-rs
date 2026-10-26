' INV: expr.addressof, stmt.declare
' SOURCE: unverified the callback needs the same grant as the Declare that receives the pointer; deny by default is a vba-rs policy
' ERROR: 70
Private Declare PtrSafe Function EnumWindows Lib "user32" (ByVal lpEnumFunc As LongPtr, ByVal lParam As LongPtr) As Long
Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Debug.Print "must not run"
    Visit = 1
End Function
Public Sub Main()
    Debug.Print EnumWindows(AddressOf Visit, 0)
End Sub
