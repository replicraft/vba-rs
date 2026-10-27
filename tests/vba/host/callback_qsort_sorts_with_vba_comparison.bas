' INV: expr.addressof, stmt.declare, vbal.5.6.16.8
' SOURCE: unverified qsort of the C runtime sorts in place by the comparison procedure; the emulated msvcrt shim is not compared with Windows
' EXPECT: -3 0 1 5 9 9
' GRANT: native-declare
Private Declare PtrSafe Sub qsort Lib "msvcrt" (ByRef base() As Long, ByVal n As LongPtr, ByVal size As LongPtr, ByVal cmp As LongPtr)
Private Declare PtrSafe Sub CopyMemory Lib "kernel32" Alias "RtlMoveMemory" (ByRef dst As Long, ByVal src As LongPtr, ByVal n As LongPtr)

Public Function Cmp(ByVal a As LongPtr, ByVal b As LongPtr) As Long
    Dim x As Long, y As Long
    CopyMemory x, a, 4
    CopyMemory y, b, 4
    If x < y Then
        Cmp = -1
    ElseIf x > y Then
        Cmp = 1
    End If
End Function

Public Sub Main()
    Dim v(0 To 5) As Long, i As Long, s As String
    v(0) = 5: v(1) = -3: v(2) = 9: v(3) = 0: v(4) = 9: v(5) = 1
    qsort v, 6, 4, AddressOf Cmp
    For i = 0 To 5
        s = s & v(i) & " "
    Next
    Debug.Print Trim$(s)
End Sub
