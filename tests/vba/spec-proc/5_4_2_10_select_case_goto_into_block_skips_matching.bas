' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 a jump into a case block evaluates neither select nor clauses
' EXPECT: in-case
' EXPECT: sel-calls=0
' EXPECT: end
Dim calls As Long
Function Sel() As Long
    calls = calls + 1
    Sel = 1
End Function
Public Sub Main()
    GoTo Inside
    Select Case Sel()
        Case 1
            Debug.Print "no"
        Case 2
Inside:
            Debug.Print "in-case"
        Case 3
            Debug.Print "no3"
    End Select
    Debug.Print "sel-calls=" & CStr(calls)
    Debug.Print "end"
End Sub
