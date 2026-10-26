' INV: vbal.5.4.2.6, stmt.do-loop
' SOURCE: spec ms-vbal 5.4.2.6 condition is evaluated before each pass (Do While) / after each pass (Loop While)
' EXPECT: pre-evals=4 n=3
' EXPECT: post-evals=3 m=3
Dim evals As Long
Function Check(ByVal ok As Boolean) As Boolean
    evals = evals + 1
    Check = ok
End Function
Public Sub Main()
    Dim n As Long, m As Long
    Do While Check(n < 3)
        n = n + 1
    Loop
    Debug.Print "pre-evals=" & CStr(evals) & " n=" & CStr(n)
    evals = 0
    Do
        m = m + 1
    Loop While Check(m < 3)
    Debug.Print "post-evals=" & CStr(evals) & " m=" & CStr(m)
End Sub
