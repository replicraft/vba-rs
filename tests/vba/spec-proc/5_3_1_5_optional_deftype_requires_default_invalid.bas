' INV: vbal.5.3.1.5
' SOURCE: spec ms-vbal 5.3.1.5 optional parameter typed only by a Def directive needs a default value
' XFAIL: wrong not implemented: Optional parameter default rule under Def directives
' ERROR: load
DefLng A-Z
Sub S(Optional a)
End Sub
Public Sub Main()
End Sub
