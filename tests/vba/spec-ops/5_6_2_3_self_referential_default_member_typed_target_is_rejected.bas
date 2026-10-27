' INV: vbal.5.6.2.3, expr.default-member
' SOURCE: derived 5.6.2.3 allows an implementation-defined limit on default member recursion; a default member that returns its own class never reaches a value, so the limit rejects the assignment to a Long at analysis instead of descending without end
' METHOD: by construction: the chain never ends, any finite limit is exceeded
' ERROR: load
'#MODULE Rec class
Public Property Get Me2() As Rec
Attribute Me2.VB_UserMemId = 0
    Set Me2 = Me
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim r As New Rec
    Dim n As Long
    n = r
End Sub
