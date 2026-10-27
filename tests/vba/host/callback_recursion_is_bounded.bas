' INV: expr.addressof, stmt.declare, vbal.5.6.16.8
' SOURCE: unverified a callback that calls the function that called it again is bounded by the call-depth and native-stack budgets
' ERROR: limit
' GRANT: native-declare
Private Declare PtrSafe Sub qsort Lib "msvcrt" (ByRef base() As Long, ByVal n As LongPtr, ByVal size As LongPtr, ByVal cmp As LongPtr)
Public Function Cmp(ByVal a As LongPtr, ByVal b As LongPtr) As Long
    Dim w(0 To 1) As Long
    qsort w, 2, 4, AddressOf Cmp
End Function
Public Sub Main()
    Dim v(0 To 1) As Long
    On Error Resume Next
    qsort v, 2, 4, AddressOf Cmp
    Debug.Print "trapped"
End Sub
