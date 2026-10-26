' INV: expr.addressof, stmt.declare
' SOURCE: unverified an error a callback does not handle stops the call and is raised by the Declare statement that made it (vba-rs design; real VBA has no defined behavior across native frames)
' EXPECT: 1234 boom 2
' GRANT: native-declare
Private Declare PtrSafe Sub qsort Lib "msvcrt" (ByRef base() As Long, ByVal n As LongPtr, ByVal size As LongPtr, ByVal cmp As LongPtr)
Dim calls As Long
Public Function Cmp(ByVal a As LongPtr, ByVal b As LongPtr) As Long
    calls = calls + 1
    If calls = 2 Then Err.Raise 1234, "cb", "boom"
End Function
Public Sub Main()
    Dim v(0 To 3) As Long
    On Error Resume Next
    qsort v, 4, 4, AddressOf Cmp
    Debug.Print Err.Number & " " & Err.Description & " " & calls
End Sub
