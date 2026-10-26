' INV: expr.addressof, stmt.declare
' SOURCE: unverified a callback runs under the step budget of the session; the limit is not trappable
' ERROR: limit
' GRANT: native-declare
' MAX_STEPS: 20000
Private Declare PtrSafe Sub qsort Lib "msvcrt" (ByRef base() As Long, ByVal n As LongPtr, ByVal size As LongPtr, ByVal cmp As LongPtr)
Public Function Cmp(ByVal a As LongPtr, ByVal b As LongPtr) As Long
    Do
    Loop
End Function
Public Sub Main()
    Dim v(0 To 3) As Long
    On Error Resume Next
    qsort v, 4, 4, AddressOf Cmp
    Debug.Print "trapped"
End Sub
