' INV: vbal.5.4.2.2
' SOURCE: spec ms-vbal 5.4.2.2 While tests its condition before every iteration
' EXPECT: n=3 cond-evals=4
' EXPECT: zero-iterations=0
Dim evals As Long
Function Lt3(ByVal n As Long) As Boolean
    evals = evals + 1
    Lt3 = (n < 3)
End Function
Public Sub Main()
    Dim n As Long, z As Long
    While Lt3(n)
        n = n + 1
    Wend
    Debug.Print "n=" & CStr(n) & " cond-evals=" & CStr(evals)
    While False
        z = z + 1
    Wend
    Debug.Print "zero-iterations=" & CStr(z)
End Sub
