' INV: vbal.5.6.16.3, stmt.if-then-else
' SOURCE: spec ms-vbal 5.6.16.3 boolean expression with data value Null is False; otherwise Let-coerced to Boolean
' EXPECT: if-null-false
' EXPECT: not-null-also-false
' EXPECT: nonzero-true
' EXPECT: zero-false
' EXPECT: while-null-skipped
' EXPECT: elseif-null-skipped
Public Sub Main()
    Dim n As Variant
    n = Null
    If n Then
        Debug.Print "BAD"
    Else
        Debug.Print "if-null-false"
    End If
    If Not n Then
        Debug.Print "BAD"
    Else
        Debug.Print "not-null-also-false"
    End If
    If 5 Then Debug.Print "nonzero-true"
    If 0 Then Debug.Print "BAD" Else Debug.Print "zero-false"
    While n
        Debug.Print "BAD"
        n = Empty
    Wend
    Debug.Print "while-null-skipped"
    If False Then
        Debug.Print "BAD"
    ElseIf n Then
        Debug.Print "BAD"
    Else
        Debug.Print "elseif-null-skipped"
    End If
End Sub
