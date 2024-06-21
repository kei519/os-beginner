int strcmp(const char *a, const char *b) {
	int i = 0;
	for (; a[i] != 0 && b[i] != 0; i++) {
		if (a[i] != b[i]) {
			return a[i] - b[i];
		}
	}
	return a[i] - b[i];
}

long atol(const char *s) {
	long v = 0;
	for (int i = 0; s[i] != 0; i++) {
		v = v * 10 + s[i] - '0';
	}
	return v;
}

int stack_ptr;
long stack[100];

long Pop() {
	return stack[stack_ptr--];
}

void Push(long value) {
	stack[++stack_ptr] = value;
}

extern "C" int main(int argc, char **argv) {
	stack_ptr = -1;

	for (int i = 1; i < argc; i++) {
		if (strcmp(argv[i], "+") == 0) {
			auto b = Pop();
			auto a = Pop();
			Push(a + b);
		} else if (strcmp(argv[i], "-") == 0) {
			auto b = Pop();
			auto a = Pop();
			Push(a - b);
		} else {
			Push(atol(argv[i]));
		}
	}

	if (stack_ptr < 0) {
		return 0;
	} else {
		return static_cast<int>(Pop());
	}
}
